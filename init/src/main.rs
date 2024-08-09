#![no_main]
#![no_std]

use alloc::string::{String, ToString};
use uefi::prelude::*;
use uefi::{print, println};
use bunnyos_common::{transfer_system_table, CoreServices, DEFAULT_SHELL};

extern crate alloc;

#[entry]
fn main(_image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();
    let mut core;

    unsafe {
        core = CoreServices::init(system_table, true);
        let st = core.get_system_table();

        transfer_system_table(st.unsafe_clone(), _image.clone(), build_info::format!(
            "Version: {} {}\nCompiler: {}\nRevision: {}",
            $.crate_info.name, $.crate_info.version, $.compiler, $.timestamp
        ).to_string());
    }

    if let Ok(_) = core.get_shared_variable("BunnyOS.Init") {
        panic!("Attempted to start more than one init.");
    }

    core.set_shared_variable("BunnyOS.Init",
        build_info::format!("{}", $.crate_info.version).as_bytes())
        .unwrap();

    let mut path = String::from(DEFAULT_SHELL);

    loop {
        println!();

        if let Err(e) = core.execute_kmode_binary(&path, false) {
            match e {
                _ => {
                    println!("Could not run command interpreter at {path}.");
                    loop {
                        print!("Please enter the path to a valid command interpreter: ");
                        path = core.readline();
                        if path.trim() != "" {
                            break;
                        }
                    }
                }
            }
        } else {
            panic!("User mode interface has died.");
        }
    }
}
