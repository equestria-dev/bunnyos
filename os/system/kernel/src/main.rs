#![no_main]
#![no_std]

use alloc::format;
use uefi::prelude::*;
use uefi::{print, println};
use russet_common::{CoreServices, OS_VERSION};
use alloc::string::ToString;

extern crate alloc;

#[entry]
#[allow(unused_must_use)]
fn main(_image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();
    let mut core;

    unsafe {
        core = CoreServices::init(system_table, true);
        core.transfer_system_table(_image.clone(), build_info::format!(
            "Version: {} {}\nCompiler: {}\nRevision: {}",
            $.crate_info.name, $.crate_info.version, $.compiler, $.timestamp
        ).to_string());
    }

    if let Ok(_) = core.get_shared_variable("Russet.Version") {
        panic!("SET_OF_INVALID_CONTEXT");
    }

    core.set_shared_variable("Russet.Version",
        build_info::format!("{}", $.crate_info.version).as_bytes())
        .unwrap();

    let os_string = format!("Russet {OS_VERSION} {}", &build_info::format!("{} {} {}-{}/{} rustc-{}", $.timestamp, $.target.cpu.arch, $.crate_info.name, $.crate_info.version, $.profile, $.compiler.version));
    core.set_shared_variable("Russet.OSString", os_string.as_bytes());
    println!("{os_string}");
    print!("Running on {} {} (HAL {})", core.firmware_vendor(), core.firmware_revision(), core.uefi_revision());

    core.execute_kmode_binary("/System/Init", true);
    panic!("CRITICAL_PROCESS_DIED");
}
