// Copyright 2026 Abdul Wahab Melethil Shibu (cychronex-labs)
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-APACHE> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate alloc;

use crate::{
    interface::{Exlex, ExlexSection},
    parser::{ErrorCode, ExlexError, Result, hash},
};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::{
    fmt::Write,
    ops::{Deref, DerefMut},
};

pub struct ExlexArena(pub String);

impl Deref for ExlexArena {
    type Target = String;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for ExlexArena {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct ExlexMutator<'a, 'b> {
    core: &'a Exlex<'a>,
    // May live only short time unlike the core which is initialization
    arena: &'b mut ExlexArena,
    // Stores new values &str
    updated_keys_vals: Vec<[usize; 2]>,
    updated_keys_hashes: Vec<u64>,
    // 1:1 with updated_keys_vals. Each element gives the index of the key in original vector
    updated_key_indices: Vec<usize>,
    // Old key sections
    updated_keys_section_ids: Vec<usize>,
    // Includes KEYS OF CORE!
    dead_core_keys: Vec<bool>,
    dead_new_keys: Vec<bool>,

    // New keys
    new_keys: Vec<[usize; 2]>,
    new_keys_hashes: Vec<u64>,
    new_keys_section_ids: Vec<usize>,
    new_values: Vec<[usize; 2]>,
    new_sections: Vec<&'a str>,
    new_sections_hashes: Vec<u64>,
    new_sections_parent_ids: Vec<usize>,
    dead_sections: Vec<bool>,
    parent_tracker: Vec<usize>,
    write_buffer: &'b mut String,
}

impl<'a, 'b> ExlexMutator<'a, 'b> {
    pub fn new(
        exlex: &'a Exlex<'a>, // FIXED: Exlex uses the long-lived 'a foundation
        data: &'b mut ExlexArena,
        write_buffer: &'b mut String, // FIXED: write_buffer uses the short-lived 'b scaffolding
    ) -> Self {
        ExlexMutator {
            core: exlex,
            arena: data,
            updated_keys_vals: Vec::new(),
            updated_keys_hashes: Vec::new(),
            updated_key_indices: Vec::new(),
            updated_keys_section_ids: Vec::new(),
            new_keys: Vec::new(),
            new_keys_hashes: Vec::new(),
            new_keys_section_ids: Vec::new(),
            new_values: Vec::new(),
            new_sections: Vec::new(),
            new_sections_hashes: Vec::new(),
            new_sections_parent_ids: Vec::new(),
            dead_core_keys: vec![false; exlex.prop_keys.len()],
            dead_new_keys: Vec::new(),
            dead_sections: vec![false; exlex.sections.len()],
            parent_tracker: exlex.parent_tracker.clone(),
            write_buffer: write_buffer,
        }
    }
    fn key_was_updated(&self, key: &str, section_id: usize) -> usize {
        let mut offset = 0;
        let hashed_key = hash(key);
        while let Some(rel_matched_idx) = self.updated_keys_hashes[offset..]
            .iter()
            .position(|&h| h == hashed_key)
        {
            let actual_idx = offset + rel_matched_idx;
            // section_id match, Key match
            if self.updated_keys_section_ids[actual_idx] == section_id
                && self.core.prop_keys[self.updated_key_indices[actual_idx]] == key
            {
                return actual_idx;
            }
            // Next slice will continue from the next hash as the macthed hashes did not meet the requirements
            offset = actual_idx + 1;
        }
        return usize::MAX;
    }

    fn is_new_key(&self, key: &str, section_id: usize) -> usize {
        let mut offset = 0;
        let hashed_key = hash(key);
        while let Some(rel_matched_idx) = self.new_keys_hashes[offset..]
            .iter()
            .position(|&h| h == hashed_key)
        {
            let actual_idx = offset + rel_matched_idx;
            let [key_start, key_end] = self.new_keys[actual_idx];
            if self.new_keys_section_ids[actual_idx] == section_id
                && &self.arena[key_start..key_end] == key
            {
                return actual_idx;
            }
            offset = actual_idx + 1;
        }
        return usize::MAX;
    }
    // returns usize::MAX if not found
    fn is_new_section(&self, section_name: &str, parent_id: usize) -> usize {
        let mut offset = 0;
        let hashed_section = hash(section_name);
        while let Some(rel_matched_idx) = self.new_sections_hashes[offset..]
            .iter()
            .position(|&h| h == hashed_section)
        {
            let actual_idx = offset + rel_matched_idx;
            if self.new_sections[actual_idx] == section_name
                && self.new_sections_parent_ids[actual_idx] == parent_id
            {
                return actual_idx;
            }
            offset = actual_idx + 1;
        }
        return usize::MAX;
    }
    fn is_core_section(&self, section_name: &str, parent_id: usize) -> usize {
        let mut offset = 0;
        let hashed_section = hash(section_name);
        while let Some(rel_matched_idx) = self.core.sections_hash[offset..]
            .iter()
            .position(|&h| h == hashed_section)
        {
            let actual_idx = offset + rel_matched_idx;
            if self.core.sections[actual_idx] == section_name
                && self.parent_tracker[actual_idx] == parent_id
            {
                return actual_idx;
            }
            offset = actual_idx + 1;
        }
        return usize::MAX;
    }
    // Partition aware returns
    fn key_in_core(&self, key: &str, section_id: usize) -> usize {
        let mut start = self.core.properties_tracker[section_id];
        let end = self.core.properties_tracker[section_id + 1];
        let hashed_key = hash(key);
        while let Some(rel_matched_idx) = self.core.properties_hash[start..end]
            .iter()
            .position(|&h| h == hashed_key)
        {
            let actual_idx = start + rel_matched_idx;
            if self.core.prop_keys[actual_idx] == key {
                return actual_idx;
            }
            start = start + rel_matched_idx + 1;
        }
        return usize::MAX;
    }

    pub fn update_prop(&mut self, key: &str, value: &str, section: ExlexSection) {
        // Check if user is trying to update an newly created key
        let section_id = section.0;
        let new_key_idx = self.is_new_key(key, section_id);
        let val_start = self.arena.len();
        self.arena.push_str(value);
        let val_end = self.arena.len();
        if new_key_idx != usize::MAX {
            self.new_values[new_key_idx] = [val_start, val_end];
        } else {
            // Check if user is trying to update an that was updated before
            let updated_key_idx = self.key_was_updated(key, section_id);
            if updated_key_idx != usize::MAX {
                self.updated_keys_vals[updated_key_idx] = [val_start, val_end];
            } else {
                // Check if the key exists in core so we only need to update value
                let key_idx = self.key_in_core(key, section_id);
                if key_idx != usize::MAX {
                    self.updated_keys_hashes
                        .push(self.core.properties_hash[key_idx]);
                    self.updated_keys_vals.push([val_start, val_end]);
                    self.updated_key_indices.push(key_idx);
                    self.updated_keys_section_ids.push(section_id);
                    self.dead_core_keys[key_idx] = false;
                } else {
                    // Create entirely new key,value
                    let key_start = self.arena.len();
                    self.arena.push_str(key);
                    let key_end = self.arena.len();
                    self.new_keys.push([key_start, key_end]);
                    self.new_keys_hashes.push(hash(key));
                    self.new_keys_section_ids.push(section_id);
                    self.new_values.push([val_start, val_end]);
                    self.dead_new_keys.push(false);
                }
            }
        }
    }

    pub fn delete_property(&mut self, key: &str, section: ExlexSection) -> Result<()> {
        let section_id = section.0;
        let updated_key_idx = self.key_was_updated(key, section_id);
        if updated_key_idx != usize::MAX {
            let actual_idx = self.updated_key_indices[updated_key_idx];
            self.dead_core_keys[actual_idx] = true;
            Ok(())
        } else {
            let key_idx = self.key_in_core(key, section_id);
            if key_idx != usize::MAX {
                self.dead_core_keys[key_idx] = true;
                Ok(())
            } else {
                let new_key_idx = self.is_new_key(key, section_id);
                if new_key_idx != usize::MAX {
                    self.dead_new_keys[new_key_idx] = true;
                    Ok(())
                } else {
                    Err(ExlexError {
                        code: ErrorCode::PropertyNotFound,
                        index: new_key_idx,
                    })
                }
            }
        }
    }
    fn write_existing_props(&mut self, section_id: usize) {
        if section_id >= self.core.sections.len() {
            return; // new sections have no core properties
        }
        let existing_props_offset = self.core.properties_tracker[section_id];
        let existing_props = &self.core.prop_keys
            [existing_props_offset..self.core.properties_tracker[section_id + 1]];
        let mut index = 0;
        while index < existing_props.len() {
            let actual_index = existing_props_offset + index;
            if self.dead_core_keys[actual_index] {
                index += 1;
                continue;
            }
            let key = existing_props[index];

            // If there is an element matching index in updated_key_indices it means The key has another value
            let value = if let Some(matched_index) = self
                .updated_key_indices
                .iter()
                .position(|&i| i == actual_index)
            {
                &self.arena[self.updated_keys_vals[matched_index][0]
                    ..self.updated_keys_vals[matched_index][1]]
            } else {
                self.core.prop_values[actual_index]
            };
            write!(self.write_buffer, "\"").unwrap();
            write!(self.write_buffer, "{}", key).unwrap();
            write!(self.write_buffer, "\"").unwrap();
            write!(self.write_buffer, ": ").unwrap();
            write!(self.write_buffer, "\"").unwrap();
            write!(self.write_buffer, "{}", value).unwrap();
            write!(self.write_buffer, "\"\n").unwrap();
            index += 1;
        }
    }

    fn write_new_props(&mut self, section_id: usize) {
        let mut offset = 0;
        while let Some(new_key_idx) = self.new_keys_section_ids[offset..]
            .iter()
            .position(|&sect_id| section_id == sect_id)
        {
            let actual_idx = offset + new_key_idx;
            if new_key_idx < self.dead_new_keys.len() && self.dead_new_keys[actual_idx] {
                offset = actual_idx + 1;
                continue; // Skip the dead key
            }
            let [key_start, key_end] = self.new_keys[actual_idx];
            let [val_start, val_end] = self.new_values[actual_idx];
            let key = &self.arena[key_start..key_end];
            let value = &self.arena[val_start..val_end];
            write!(self.write_buffer, "\"").unwrap();
            write!(self.write_buffer, "{}", key).unwrap();
            write!(self.write_buffer, "\"").unwrap();
            write!(self.write_buffer, ": ").unwrap();
            write!(self.write_buffer, "\"").unwrap();
            write!(self.write_buffer, "{}", value).unwrap();
            write!(self.write_buffer, "\"\n").unwrap();
            offset = actual_idx + 1;
        }
    }
    fn write_section(&mut self, sect_idx: usize) {
        if sect_idx != 0 {
            write!(self.write_buffer, "sect \"").unwrap();
            if sect_idx >= self.core.sections.len() {
                write!(
                    self.write_buffer,
                    "{}",
                    self.new_sections[sect_idx - self.core.sections.len()]
                )
                .unwrap();
            } else {
                write!(self.write_buffer, "{}", self.core.sections[sect_idx]).unwrap();
            }
            write!(self.write_buffer, "\" {{\n").unwrap();
        }
        self.write_existing_props(sect_idx);
        self.write_new_props(sect_idx);

        // First section is ROOT, so skip it because its parent is 0.
        let mut offset = 1;
        while let Some(idx) = self.parent_tracker[offset..]
            .iter()
            .position(|&parent_idx| parent_idx == sect_idx)
        {
            let actual_idx = offset + idx;
            // offset starts at one so actual_idx may exceed dead_sections
            if actual_idx < self.dead_sections.len() && !self.dead_sections[actual_idx] {
                self.write_section(actual_idx);
            }
            offset = actual_idx + 1;
        }
        if sect_idx != 0 {
            write!(self.write_buffer, "}}\n").unwrap();
        }
    }
    pub fn new_section(&mut self, section_name: &'a str, parent: ExlexSection) -> Result<()> {
        let parent_id = parent.0;
        if self.is_new_section(section_name, parent_id) != usize::MAX {
            Err(ExlexError {
                code: ErrorCode::AlreadyCreatedSection,
                index: usize::MAX,
            })
        } else {
            if self.is_core_section(section_name, parent_id) != usize::MAX {
                Err(ExlexError {
                    code: ErrorCode::DuplicateSectionsNotAllowed,
                    index: usize::MAX,
                })
            } else {
                self.new_sections.push(section_name);
                self.new_sections_hashes.push(hash(section_name));
                self.parent_tracker.push(parent_id);
                self.dead_sections.push(false);
                self.new_sections_parent_ids.push(parent_id);
                Ok(())
            }
        }
    }
    pub fn move_section(&mut self, section_id: usize, to_parent: ExlexSection) {
        self.parent_tracker[section_id] = to_parent.0;
    }
    pub fn delete_section(&mut self, section: ExlexSection) {
        self.dead_sections[section.0] = true;
    }

    pub fn save(&mut self) {
        self.write_buffer.clear();
        self.write_section(0);
    }
}
