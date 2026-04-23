// Copyright 2026 Abdul Wahab Melethil Shibu (cychronex-labs)
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-APACHE> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use crate::parser::{ErrorCode, ExlexError, ExlexParser, Result, hash};
use crate::writer::{ExlexArena, ExlexMutator};
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug)]
pub struct Exlex<'a> {
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
#[derive(Clone, Copy)]
pub struct ExlexSection(pub usize);

impl<'a> Exlex<'a> {
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
    pub fn init_mutator<'b>(
        &'a self,
        arena: &'b mut ExlexArena,
        write_buffer: &'b mut String,
    ) -> Result<ExlexMutator<'a, 'b>> {
        Ok(ExlexMutator::new(self, arena, write_buffer))
    }
    pub fn get_root(&self) -> ExlexSection {
        ExlexSection(0)
    }
    pub fn get_child(&self, child: &str, parent: ExlexSection) -> Result<ExlexSection> {
        let parent_id = parent.0;
        let hashed_sect_name = hash(child);
        let children_start = self.children_tracker[parent_id][0];
        let children_slice =
            &self.sections_hash[children_start..self.children_tracker[parent_id][1]];

        let mut offset = 0;
        while let Some(rel_idx) = children_slice[offset..]
            .iter()
            .position(|&sect_hash| hashed_sect_name == sect_hash)
        {
            let actual_idx = children_start + offset + rel_idx;
            if self.sections[actual_idx] == child {
                return Ok(ExlexSection(actual_idx));
            }
            offset += rel_idx + 1;
        }
        Err(ExlexError {
            code: ErrorCode::SectionNotFound,
            index: usize::MAX,
        })
    }
    pub fn get_child_path(&self, path: &[&str], start_node: ExlexSection) -> Result<ExlexSection> {
        let mut current = start_node;
        for &node_name in path {
            current = self.get_child(node_name, current)?;
        }
        Ok(current)
    }
    pub fn iter_section_properties(
        &self,
        section: ExlexSection,
    ) -> impl Iterator<Item = (&'a str, &'a str)> {
        let section_id = section.0;
        let start = self.properties_tracker[section_id];
        let end = self.properties_tracker[section_id + 1];
        self.prop_keys[start..end]
            .iter()
            .copied()
            .zip(self.prop_values[start..end].iter().copied())
    }
    pub fn get_property(&self, key: &str, section: ExlexSection) -> Result<&str> {
        let key_hash = hash(key);
        let section_id = section.0;
        if section_id >= self.sections.len() {
            return Err(ExlexError {
                code: ErrorCode::SectionNotFound,
                index: usize::MAX,
            });
        } else {
            let properties_start = self.properties_tracker[section_id];
            let properties_end = self.properties_tracker[section_id + 1];
            let property_slice = &self.properties_hash[properties_start..properties_end];
            let mut offset = 0;
            while let Some(rel_idx) = property_slice[offset..]
                .iter()
                .position(|&prop_hash| key_hash == prop_hash)
            {
                let actual_idx = properties_start + offset + rel_idx;
                if self.prop_keys[actual_idx] == key {
                    return Ok(self.prop_values[actual_idx]);
                }
                offset += rel_idx + 1;
            }
        }
        Err(ExlexError {
            code: ErrorCode::PropertyNotFound,
            index: usize::MAX,
        })
    }
    pub fn get_property_as<T: core::str::FromStr>(
        &self,
        key: &str,
        section: ExlexSection,
    ) -> Result<T> {
        let val_str = self.get_property(key, section)?;
        val_str.parse::<T>().map_err(|_| ExlexError {
            code: ErrorCode::MalformedLiteral,
            index: usize::MAX,
        })
    }
    pub fn get_nested_property(&self, path: &[&str], key: &str) -> Result<&str> {
        let section = self.get_child_path(path, self.get_root())?;
        self.get_property(key, section)
    }
}
