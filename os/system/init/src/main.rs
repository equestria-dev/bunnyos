#![no_main]
#![no_std]

use alloc::format;
use alloc::string::{String, ToString};
use uefi::prelude::*;
use uefi::{print, println};
use russet_common::{CoreServices, DEFAULT_SHELL};

extern crate alloc;

#[entry]
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

    if let Ok(_) = core.get_shared_variable("Russet.Init") {
        panic!("UNEXPECTED_INITIALIZATION_CALL");
    }

    core.set_shared_variable("Russet.Init",
        build_info::format!("{}", $.crate_info.version).as_bytes())
        .unwrap();

    let mut path = String::from(DEFAULT_SHELL);

    loop {
        println!();

        let string = format!("\\rootfs{}", path.replace("/", "\\"));
        if core.execute_user_binary(&string).is_err() {
            println!("\nThe command interpreter at \"{path}\" could not be started.");
            loop {
                print!("Please enter the path to a valid command interpreter: ");
                path = core.readline();
                if path.trim() != "" {
                    break;
                }
            }
        } else {
            panic!("NO_USER_MODE_CONTEXT");
        }
    }
}
