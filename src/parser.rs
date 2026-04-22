// Copyright 2026 Abdul Wahab Melethil Shibu (cychronex-labs)
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-APACHE> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use alloc::vec;
use alloc::vec::Vec;
use memchr::{memchr2, memchr2_iter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExlexError {
    pub code: ErrorCode,
    pub index: usize, // The exact byte offset where the error occurred
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ErrorCode {
    InvalidSyntax,
    InvalidCharacter,
    InvalidBracket,
    UnclosedQuote,
    MalformedLiteral,
    PropertyNotFound,
    SectionParentNotFound,
    SectionNotFound,
    DuplicateSectionsNotAllowed,
    InvalidSection,
    IoError,
    AlreadyCreatedSection,
}

pub type Result<T> = core::result::Result<T, ExlexError>;

pub(crate) struct ExlexParser<'a> {
    pub(crate) data: &'a str,
    data_as_bytes: &'a [u8],
    section_id: usize,
    // A vector of sections first section ROOT is at index 0 and every user defined section starts from 1
    pub(crate) sections: Vec<&'a str>,
    // A vector of all keys in the order of their definition
    pub(crate) prop_keys: Vec<&'a str>,
    // A vector of all values in the order of their definition
    pub(crate) prop_values: Vec<&'a str>,
    // A vector of all property's key hash in the order of their definition and same for sections
    pub(crate) properties_hash: Vec<u64>,
    pub(crate) sections_hash: Vec<u64>,
    // Properties Tracker
    // A vector that tracks how many properties has been defined from start of ROOT to next section and so on
    // START_OF_PROPERTY_OF_SECTION(x) = SPS(x)
    // [0,SPS(1),SPS(2),SPS(3),SPS(4)]
    // Because rust's range are exclusive to get properties defined inside section 2 we only need to:
    //                                  Section_Id of what we want
    //                                    |
    //     let start = properties_tracker[2];
    //
    //                                 Section_Id of the next section
    //                                  |
    //     let end = properties_tracker[3];
    //     properties[start..end]
    pub(crate) properties_tracker: Vec<usize>,
    // Children tracker
    //[Start]
    // ROOT
    //    |       |
    // [ [1, 9], [1, 3] ]
    //       |       |
    //      ROOT
    //     [End]
    pub(crate) children_tracker: Vec<[usize; 2]>,
    active_parents: Vec<usize>,
    pub(crate) parent_tracker: Vec<usize>,
    cursor: usize,
    props_parsed: usize,
}

// FxHash
#[inline(always)]
pub(crate) fn hash(key: &str) -> u64 {
    const K: u64 = 0x517cc1b727220a95;
    let mut hash: u64 = 0;
    let bytes = key.as_bytes();
    let mut chunks = bytes.chunks_exact(8);
    for chunk in &mut chunks {
        let word = u64::from_ne_bytes(chunk.try_into().unwrap());
        hash = (hash.rotate_left(5) ^ word).wrapping_mul(K);
    }
    for &byte in chunks.remainder() {
        hash = (hash.rotate_left(5) ^ (byte as u64)).wrapping_mul(K);
    }
    hash
}
impl<'a> ExlexParser<'a> {
    pub(crate) fn new(
        data: &'a str,
        data_as_bytes: &'a [u8],
        disable_preallocator: Option<bool>,
        preallocate_sections: Option<usize>,
        preallocate_props: Option<usize>,
        preallocate_max_depth: Option<usize>,
    ) -> Self {
        let mut prop_count = preallocate_props.unwrap_or(0);
        let mut section_count = preallocate_sections.unwrap_or(1);
        let depth = preallocate_max_depth.unwrap_or(5);
        if disable_preallocator.unwrap_or(false) == false {
            section_count += 1; // add space for ROOT!
            for idx in memchr2_iter(b':', b'{', data_as_bytes) {
                if data_as_bytes[idx] == b':' {
                    prop_count += 1
                } else if data_as_bytes[idx] == b'{' {
                    section_count += 1
                }
            }
        }
        let mut parser = ExlexParser {
            data,
            data_as_bytes: data_as_bytes,
            section_id: 0,
            prop_keys: Vec::with_capacity(prop_count),
            prop_values: Vec::with_capacity(prop_count),
            properties_tracker: Vec::with_capacity(prop_count),
            properties_hash: Vec::with_capacity(prop_count),
            sections: Vec::with_capacity(section_count),
            sections_hash: Vec::with_capacity(section_count),
            children_tracker: Vec::with_capacity(section_count),
            active_parents: Vec::with_capacity(depth),
            parent_tracker: vec![0],
            cursor: 0,
            props_parsed: 0,
        };
        // Initialize
        parser.sections.push("ROOT");
        parser.sections_hash.push(hash("ROOT"));
        parser.active_parents.push(0);
        parser.properties_tracker.push(0);
        parser.children_tracker.push([0, 0]);
        parser
    }

    fn property_parser(&mut self) -> Result<()> {
        // Extract the property keys
        let [prop_key_start, prop_key_end] =
            extract_quoted_literals(self.data_as_bytes, &mut self.cursor)?;
        // Skip spaces or such if exists
        skip_to_next_meaningful_bytes(self.data_as_bytes, &mut self.cursor);
        // Next must be colon
        if self.cursor >= self.data_as_bytes.len() {
            return Err(ExlexError {
                code: ErrorCode::InvalidSyntax,
                index: self.cursor,
            });
        }
        if self.data_as_bytes[self.cursor] == b':' {
            // Skip colon
            self.cursor += 1;
            // Skip spaces again
            skip_to_next_meaningful_bytes(self.data_as_bytes, &mut self.cursor);
            if self.cursor >= self.data_as_bytes.len() {
                return Err(ExlexError {
                    code: ErrorCode::InvalidSyntax,
                    index: self.cursor,
                });
            }
            // Next character must be "
            if self.data_as_bytes[self.cursor] == b'"' {
                let [value_start, value_end] =
                    extract_quoted_literals(self.data_as_bytes, &mut self.cursor)?;
                // Update properties
                let key = &self.data[prop_key_start..prop_key_end];
                let value = &self.data[value_start..value_end];
                self.prop_keys.push(key);
                self.properties_hash.push(hash(key));
                // Update values
                self.prop_values.push(value);
                self.props_parsed += 1;
                return Ok(());
            } else {
                return Err(ExlexError {
                    code: ErrorCode::InvalidSyntax,
                    index: self.cursor,
                });
            }
        } else {
            return Err(ExlexError {
                code: ErrorCode::InvalidSyntax,
                index: self.cursor,
            });
        }
    }
    fn section_identify(&mut self) -> Result<()> {
        if self.cursor + 4 < self.data_as_bytes.len()
            && self.data_as_bytes[self.cursor..self.cursor + 4] == *b"sect"
        {
            self.cursor += 4;
            skip_to_next_meaningful_bytes(self.data_as_bytes, &mut self.cursor);
            let [name_start, name_end] =
                extract_quoted_literals(self.data_as_bytes, &mut self.cursor)?;
            skip_to_next_meaningful_bytes(self.data_as_bytes, &mut self.cursor);
            let section_name = &self.data[name_start..name_end];
            if self.data_as_bytes[self.cursor] == b'{' {
                self.section_id += 1;
                self.properties_tracker.push(self.props_parsed);
                // We confirmed section initialization
                self.sections.push(section_name);
                self.sections_hash.push(hash(section_name));
                // Currently active section that hasnt been closed yet! Meaning this section have a parent
                // [by default everything have parent 0 which is the root]
                //let current_parent = *self.active_parents.last().expect("Stack empty!");
                // We push the current parent so that we know who is the parent of current section
                let current_parent = *self.active_parents.last().unwrap();

                // Push the actual parent ID
                self.parent_tracker.push(current_parent);
                // We push this section into active parents which which only be popped when we find a closing bracket
                self.active_parents.push(self.section_id);
                self.children_tracker.push([0, 0]);
                return Ok(());
            } else {
                return Err(ExlexError {
                    code: ErrorCode::InvalidSyntax,
                    index: self.cursor,
                });
            }
        }
        Ok(())
    }
    pub(crate) fn parse(&mut self) -> Result<()> {
        let length = self.data_as_bytes.len();
        // Why not for loop?
        // Using for loop means the for loop dictates the cursor therefore no look aheads or look backs will be possible!
        while self.cursor < length {
            let current_byte = self.data_as_bytes[self.cursor];
            match current_byte {
                // Whitespace
                b' ' | b'\r' | b'\t' => {
                    // Find the next meaningful bytes skipping all whitespace characters
                    // Function does the job of cursor+=1 so no cursor updates is needed
                    skip_to_next_meaningful_bytes(self.data_as_bytes, &mut self.cursor);
                }
                // Comments
                b'#' => {
                    // A comment's end is dictated by the newline
                    skip_until_newline_bytes(self.data_as_bytes, &mut self.cursor);
                }
                // LITERAL
                b'"' => {
                    // We hit a literal's first byte!
                    // Give it to the literal parser and let it do the job

                    self.property_parser()?;
                }

                b'}' => {
                    let closed = self.active_parents.pop();

                    if let Some(closed_id) = closed {
                        if closed_id == 0 {
                            // Popped ROOT — this is a stray closing bracket
                            return Err(ExlexError {
                                code: ErrorCode::InvalidBracket,
                                index: self.cursor,
                            });
                        }
                        // If the closed section = current section it means it have zero children so as an identifier we push [closed_section_id,closed_section_id] into the children tracker
                        let children_span = [closed_id + 1, self.section_id + 1];
                        // if thats not the case then that means the children of the closed section span:
                        //     STARTS: The closed section id + 1 [Inclusive]
                        //     ENDS AT: current section id + 1 [exclusive]

                        self.children_tracker[closed_id] = children_span;
                        self.cursor += 1;
                    } else {
                        return Err(ExlexError {
                            code: ErrorCode::InvalidBracket,
                            index: self.cursor,
                        });
                    }
                }
                _ => {
                    self.section_identify()?;
                    // If section is found then cursor is pointing at {
                    self.cursor += 1;
                }
            }
        }
        self.children_tracker[0] = [1, self.section_id + 1];
        self.properties_tracker.push(self.props_parsed);
        Ok(())
    }
}
#[inline(always)]
fn skip_to_next_meaningful_bytes(bytes: &[u8], cursor: &mut usize) {
    let length = bytes.len();
    while *cursor < length && matches!(bytes[*cursor], b' ' | b'\r' | b'\t' | b'\n') {
        *cursor += 1;
    }
}
#[inline(always)]
fn skip_until_newline_bytes(bytes: &[u8], cursor: &mut usize) {
    if let Some(index) = memchr::memchr(b'\n', &bytes[*cursor..]) {
        *cursor += index;
    }
    // Skip the cursor once more to skip the current newline too
    *cursor += 1;
}

// Find complete word cursor must be start of a quote [Quotes are supported]
fn extract_quoted_literals(bytes: &[u8], cursor: &mut usize) -> Result<[usize; 2]> {
    // Skip the quote
    *cursor += 1;
    let length = bytes.len();
    // starting of literal
    let start = *cursor;
    // Slice to Search  from bytes

    while *cursor < length {
        let search_slice = &bytes[*cursor..];
        // Find first occurrence of " or escape code
        if let Some(index) = memchr2(b'"', b'\\', search_slice) {
            // Memchr2 returns relative index because we passed a slice therefore add it to cursor
            *cursor += index;
            // Update next search slice

            let found_byte = search_slice[index];
            if found_byte == b'"' {
                // Because end is exclusive in array[start..end], we include the position of ending "
                let end = *cursor;
                // Now skip the " to prevent looping
                *cursor += 1;
                return Ok([start, end]);
            } else {
                // Skip escape code and whatever character is next
                *cursor += 2;
            }
        } else {
            break;
        }
    }
    if start == *cursor {
        return Err(ExlexError {
            code: ErrorCode::MalformedLiteral,
            index: start,
        });
    }

    // memchr returns None!
    return Err(ExlexError {
        code: ErrorCode::UnclosedQuote,
        index: *cursor,
    });
}
