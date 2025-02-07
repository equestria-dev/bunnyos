#![no_main]
#![no_std]

use alloc::string::{String, ToString};
use uefi::prelude::*;
use uefi::{print, println};
use russet_common::{CoreServices, DEFAULT_KERNEL};

extern crate alloc;

#[entry]
#[allow(unused_must_use)]
fn main(_image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();
    let mut core;

    unsafe {
        core = CoreServices::init(system_table, true);
        let mut st = core.get_system_table();

        core.transfer_system_table(_image.clone(), build_info::format!(
            "Version: {} {}\nCompiler: {}\nRevision: {}",
            $.crate_info.name, $.crate_info.version, $.compiler, $.timestamp
        ).to_string());

        let stdout = st.stdout();
        stdout.reset(false).expect("Failed to clear screen buffer");
        stdout.enable_cursor(true)
            .expect("Failed to change cursor status");
    }

    if let Ok(_) = core.get_shared_variable("Russet.Bootloader") {
        panic!("UNEXPECTED_INITIALIZATION_CALL");
    }

    core.set_shared_variable("Russet.Bootloader",
        build_info::format!("{}", $.crate_info.version).as_bytes())
        .unwrap();

    let mut path = String::from(DEFAULT_KERNEL);

    loop {
        println!("{} ({path})", &build_info::format!("rouse bootloader {}", $.crate_info.version));

        if let Err(e) = core.execute_kmode_binary(&path, false) {
            match e {
                _ => {
                    println!("\nThe kernel \"{path}\" could not be loaded at this time.");
                    loop {
                        print!("Rouse> ");
                        path = core.readline();
                        if path.trim() != "" {
                            break;
                        }
                    }
                }
            }
        } else {
            panic!("KMODE_EXCEPTION_NOT_HANDLED");
        }
    }
}
