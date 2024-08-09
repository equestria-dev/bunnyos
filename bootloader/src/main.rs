#![no_main]
#![no_std]

use alloc::string::{String, ToString};
use uefi::prelude::*;
use uefi::{print, println};
use uefi::proto::console::text::Color;
use bunnyos_common::{transfer_system_table, CoreServices, DEFAULT_KERNEL};

extern crate alloc;

#[entry]
#[allow(unused_must_use)]
fn main(_image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();
    let mut core;

    unsafe {
        core = CoreServices::init(system_table, true);
        let mut st = core.get_system_table();

        transfer_system_table(st.unsafe_clone(), _image.clone(), build_info::format!(
            "Version: {} {}\nCompiler: {}\nRevision: {}",
            $.crate_info.name, $.crate_info.version, $.compiler, $.timestamp
        ).to_string());

        let stdout = st.stdout();
        stdout.reset(false).expect("Failed to clear screen buffer");
        stdout.enable_cursor(true)
            .expect("Failed to change cursor status");
    }

    if let Ok(_) = core.get_shared_variable("BunnyOS.Bootloader") {
        panic!("Attempted to reinitialize bootloader.");
    }

    core.set_shared_variable("BunnyOS.Bootloader",
        build_info::format!("{}", $.crate_info.version).as_bytes())
        .unwrap();

    core.set_color(Color::DarkGray, Color::Black);
    print!("{}", &build_info::format!("BunnyLoader v{} - The BunnyOS boot loader\n({}, {})",
        $.crate_info.version, $.compiler, $.timestamp));

    let mut path = String::from(DEFAULT_KERNEL);

    loop {
        println!();
        core.set_color(Color::LightGray, Color::Black);

        if let Err(e) = core.execute_kmode_binary(&path, false) {
            match e {
                _ => {
                    println!("\nCannot load kernel: {path}");
                    loop {
                        print!("boot: ");
                        path = core.readline();
                        if path.trim() != "" {
                            break;
                        }
                    }
                }
            }
        } else {
            panic!("Kernel has died.");
        }
    }
}
