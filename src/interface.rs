// Copyright 2026 Abdul Wahab Melethil Shibu (cychronex-labs)
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-APACHE> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use crate::alloc::borrow::Cow;
use crate::parser::{ErrorCode, ExlexError, ExlexParser, Result, hash};
use crate::writer::ExlexMutator;
use alloc::string::String;
use alloc::vec::Vec;
use memchr::memchr;

#[derive(Debug)]
pub struct Exlex<'a> {
    pub(crate) data: &'a str,
    pub(crate) sections: Vec<&'a str>,
    pub(crate) prop_keys: Vec<&'a str>,
    pub(crate) prop_values: Vec<&'a str>,
    pub(crate) properties_tracker: Vec<usize>,
    pub(crate) properties_hash: Vec<u64>,
    pub(crate) sections_hash: Vec<u64>,
    pub(crate) children_tracker: Vec<[usize; 2]>,
    pub(crate) parent_tracker: Vec<usize>,
    // sections_name_spans: Vec<[usize; 2]>,
    // prop_keys_spans: Vec<[usize; 2]>,
    // prop_values_spans: Vec<[usize; 2]>
}

impl<'a> Exlex<'a> {
    // The only public entry point for the user
    pub fn init_reader(
        data: &'a str,
        disable_preallocator: Option<bool>,
        preallocate_sections: Option<usize>,
        preallocate_props: Option<usize>,
        preallocate_max_depth: Option<usize>,
    ) -> Result<Self> {
        let data_as_bytes = data.as_bytes();
        let mut parser = ExlexParser::new(
            data,
            data_as_bytes,
            disable_preallocator,
            preallocate_sections,
            preallocate_props,
            preallocate_max_depth,
        );
        parser.parse()?; // Run the state machine
        // Handoff the memory arrays. The parser is destroyed, the memory lives on.

        Ok(Exlex {
            data: parser.data,
            sections: parser.sections,
            prop_keys: parser.prop_keys,
            prop_values: parser.prop_values,
            properties_hash: parser.properties_hash,
            sections_hash: parser.sections_hash,
            properties_tracker: parser.properties_tracker,
            children_tracker: parser.children_tracker,
            parent_tracker: parser.parent_tracker,
        })
    }
    pub fn init_mutator(&'a self) -> Result<ExlexMutator<'a>> {
        Ok(ExlexMutator::new(self, &self.data))
    }
    pub fn total_properties(&self) -> usize {
        self.prop_keys.len()
    }
    // Returns offset, and slice of properties hashes
    pub fn get_property_hashes(&self, section: Option<&str>) -> Result<(usize, &[u64])> {
        let section_id = self.get_section_id(section.unwrap_or("ROOT"))?;
        self.get_property_hashes_by_sect_id(section_id)
    }
    pub fn get_property_hashes_by_sect_id(&self, section_id: usize) -> Result<(usize, &[u64])> {
        let start = self.properties_tracker[section_id];
        let end = self.properties_tracker[section_id + 1];
        return Ok((start, &self.properties_hash[start..end]));
    }
    pub fn get_property_id_by_sect_id(&self, key: &str, section_id: usize) -> Result<usize> {
        let key_hash = hash(key);
        // Get only the relevant part of vector as a slice
        let start = self.properties_tracker[section_id];
        let end = self.properties_tracker[section_id + 1];
        // Every property from start [inclusive] to end [exclusive] belongs to the section
        let properties_hash_slice = &self.properties_hash[start..end];
        let mut index = 0;
        for hash in properties_hash_slice {
            if hash == &key_hash && self.prop_keys[start + index] == key {
                return Ok(start + index);
            }
            index += 1;
        }

        // An offset to determine from where do we need to iter to find hash
        let mut offset = 0;
        // We get index relative to that SLICE
        while let Some(rel_index) = properties_hash_slice[offset..]
            .iter()
            .position(|&h| h == key_hash)
        {
            // Now this local_idx is index relative to the properties_hash_slice NOT The Properties
            let local_idx = offset + rel_index;
            // The actual index will be local_idx + the starting point of where these properties are of section_id
            let actual_idx = start + local_idx;
            if self.prop_keys[actual_idx] == key {
                return Ok(actual_idx);
            }
            offset = local_idx + 1;
        }
        Err(ExlexError {
            code: ErrorCode::PropertyNotFound,
            index: usize::MAX,
        })
    }
    pub fn get_property_id(&self, key: &str, section: &str) -> Result<usize> {
        let section_id = self.get_section_id(section)?;
        self.get_property_id_by_sect_id(key, section_id)
    }
    pub fn query_property(&self, key: &str) -> Result<&'a str> {
        let target_hash = &hash(key);
        let props_hash_slice = self.properties_hash.as_slice();
        let mut index = 0;
        for hash in props_hash_slice {
            if hash == target_hash && self.prop_keys[index] == key {
                return Ok(self.prop_values[index]);
            }
            index += 1;
        }
        Err(ExlexError {
            code: ErrorCode::PropertyNotFound,
            index: usize::MAX,
        })
    }

    // Support for \
    pub fn get_property(&self, key: &str, section: &str) -> Result<&'a str> {
        let id = self.get_property_id(key, section)?;
        return Ok(self.prop_values[id]);
    }

    // use ROOT to get properties that does not belong to any sections
    pub fn get_properties(&self, section: &str) -> Result<&[&'a str]> {
        let section_id = self.get_section_id(section)?;
        let start = self.properties_tracker[section_id];
        let end = self.properties_tracker[section_id + 1];
        Ok(&self.prop_keys[start..end])
    }
    pub fn get_properties_by_section_id(&self, section_id: usize) -> Result<&[&'a str]> {
        // WARNING
        // THIS METHOD IS EXCEPTIONALLY FAST BUT CAN RETURN INVALID DATA IF WRONG SECTION INDEX IS SPECIFIED
        // USE ONLY IF YOU KNOW WHAT YOU ARE DOING!
        // Excluding search for section id its same as get_properties so I no need to worry about readability
        Ok(&self.prop_keys
            [self.properties_tracker[section_id]..self.properties_tracker[section_id + 1]])
    }
    pub fn get_sections(&self, parent_section: Option<&str>) -> Result<&[&'a str]> {
        if let Some(parent) = parent_section {
            let section_id = self.get_section_id(parent)?;
            let start = self.children_tracker[section_id][0];
            let end = self.children_tracker[section_id][1];
            return Ok(&self.sections[start..end]);
        }
        Ok(&self.sections[0..])
    }

    pub fn get_section_by_id(&self, id: usize) -> &'a str {
        return self.sections[id];
    }

    pub fn get_section_ids(&self, section_name: &str) -> Vec<usize> {
        let mut section_ids = Vec::new();
        let sect_hash = hash(section_name);
        let mut idx = 0;
        while idx < self.sections_hash.len() {
            let section_hash = self.sections_hash[idx];
            if section_hash == sect_hash {
                if section_name == self.sections[idx] {
                    section_ids.push(idx);
                }
            }
            idx += 1;
        }
        return section_ids;
    }
    pub fn get_section_id(&self, query: &str) -> Result<usize> {
        // 1. Clean the query: Strip "ROOT." if it exists, because we implicitly start at ROOT.
        let sanitized_query = if query.starts_with("ROOT.") {
            &query[5..]
        } else if query == "ROOT" {
            return Ok(0);
        } else {
            query
        };

        let mut sections = sanitized_query.split('.');
        let last_section = sections.next_back();
        match last_section {
            Some(last_section_name) => {
                let last_section_hash = hash(last_section_name);
                let mut parent_index = 0;

                // 2. Route through intermediate parents
                for next_target in sections {
                    if next_target.is_empty() {
                        continue;
                    }
                    let next_target_hash = hash(next_target);
                    let mut cursor = self.children_tracker[parent_index][0];
                    let scope_end = self.children_tracker[parent_index][1];
                    let mut found = false;

                    while cursor < scope_end {
                        if self.sections_hash[cursor] == next_target_hash
                            && self.sections[cursor] == next_target
                        {
                            parent_index = cursor;
                            found = true;
                            break; // Found the parent, move one level deeper
                        }
                        // TELEPORT: Instantly skip this section and all its nested children
                        cursor = self.children_tracker[cursor][1];
                    }
                    if !found {
                        return Err(ExlexError {
                            code: ErrorCode::SectionParentNotFound,
                            index: usize::MAX,
                        });
                    } // Parent path is broken
                }

                // 3. Find the final target within the last found parent
                let mut cursor = self.children_tracker[parent_index][0];
                let scope_end = self.children_tracker[parent_index][1];

                while cursor < scope_end {
                    if self.sections_hash[cursor] == last_section_hash
                        && self.sections[cursor] == last_section_name
                    {
                        return Ok(cursor); // Found it!
                    }
                    // THE TELEPORT again
                    cursor = self.children_tracker[cursor][1];
                }
            }
            None => {
                return Err(ExlexError {
                    code: ErrorCode::InvalidSection,
                    index: usize::MAX,
                });
            }
        }

        return Err(ExlexError {
            code: ErrorCode::SectionNotFound,
            index: usize::MAX,
        });
    }
}
