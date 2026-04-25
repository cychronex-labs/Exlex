// Copyright 2026 Abdul Wahab Melethil Shibu (cychronex-labs)
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-APACHE> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

extern crate alloc;

mod interface;
mod parser;
mod writer;

pub use interface::{Exlex, ExlexSection};
pub use parser::{ErrorCode, ExlexError, Result};
pub use writer::{ExlexArena, ExlexMutator};
