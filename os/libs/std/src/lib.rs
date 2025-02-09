#![no_std]

pub extern crate alloc;

use uefi::prelude::{Boot, SystemTable};
use russet_common::CoreServices;

pub mod prelude;
mod macros;

pub use alloc::boxed;
pub use alloc::borrow;
pub use core::char;
pub use core::array;
pub use core::any;
pub use core::cell;
pub use core::clone;
pub use core::cmp;
pub use alloc::collections;
pub use core::convert;
pub use core::default;
pub use core::error;
pub use core::f32;
pub use core::f64;
pub use core::fmt;
pub use core::future;
pub use core::ffi;
pub use core::ascii;
pub use core::hash;
pub use core::hint;
pub use core::iter;
pub use core::marker;
pub use core::mem;
pub use core::num;
pub use core::ops;
pub use core::option;
pub use core::panic;
pub use core::pin;
pub use core::prelude::*;
pub use core::primitive;
pub use core::ptr;
pub use core::result;
pub use alloc::slice;
pub use alloc::str;
pub use alloc::string;
pub use core::sync;
pub use core::task;
pub use core::time;
pub use alloc::vec;
pub use core::arch;
pub use core::assert;
pub use core::assert_eq;
pub use core::assert_ne;
pub use core::cfg;
pub use core::column;
pub use core::compile_error;
pub use core::concat;
pub use core::debug_assert;
pub use core::debug_assert_eq;
pub use core::debug_assert_ne;
pub use core::env;
pub use core::file;
pub use alloc::format;
pub use core::format_args;
pub use core::include;
pub use core::include_bytes;
pub use core::include_str;
pub use core::line;
pub use core::matches;
pub use core::module_path;
pub use core::option_env;
pub use uefi::print;
pub use uefi::println;
pub use core::stringify;
pub use core::todo;
pub use core::unimplemented;
pub use core::unreachable;
pub use core::write;
pub use core::writeln;
pub use std_detect::is_x86_feature_detected;

#[allow(unused_imports)]
pub use crate::macros::*;

use alloc::string::String;
use uefi::Handle;

#[allow(dead_code)]
pub(crate) static mut CORE_SERVICES: Option<CoreServices> = None;

pub unsafe fn init(mut system_table: SystemTable<Boot>, image: Handle) {
    uefi::helpers::init(&mut system_table).unwrap();
    let mut core = CoreServices::init(system_table, false);
    core.transfer_system_table(image.clone(), String::new());
}
