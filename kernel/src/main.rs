#![no_main]
#![no_std]

use uefi::prelude::*;
use uefi::{print, println};
use bunnyos_common::{transfer_system_table, CoreServices, OS_VERSION};
use alloc::string::ToString;
use uefi::proto::console::text::Color;

extern crate alloc;

#[entry]
#[allow(unused_must_use)]
fn main(_image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();
    let mut core;

    unsafe {
        core = CoreServices::init(system_table, true);
        let mut st = core.get_system_table();

        let cols = st.stdout().current_mode().unwrap().unwrap().columns();
        core.set_color(Color::DarkGray, Color::Black);
        print!("\n{}", "─".repeat(cols));

        transfer_system_table(st.unsafe_clone(), _image.clone(), build_info::format!(
            "Version: {} {}\nCompiler: {}\nRevision: {}",
            $.crate_info.name, $.crate_info.version, $.compiler, $.timestamp
        ).to_string());
    }

    if let Ok(_) = core.get_shared_variable("BunnyOS.Version") {
        panic!("Attempted to start kernel from user mode.");
    }

    core.set_shared_variable("BunnyOS.Version",
        build_info::format!("{}", $.crate_info.version).as_bytes())
        .unwrap();

    core.set_color(Color::Cyan, Color::Black);
    println!("\n         ,\\       ");
    print!("         \\\\\\,_    ");
    core.set_color(Color::LightCyan, Color::Black);
    println!("Welcome to BunnyOS {}!", &OS_VERSION);
    core.set_color(Color::Cyan, Color::Black);

    print!("          \\` ,\\   ");
    core.set_color(Color::DarkGray, Color::Black);
    println!("{}", &build_info::format!("{} {} ({})", $.crate_info.name, $.crate_info.version, $.profile));
    core.set_color(Color::Cyan, Color::Black);

    print!("     __,.-\" =__)  ");
    core.set_color(Color::DarkGray, Color::Black);
    println!("{}", &build_info::format!("rustc {} ({}, {})", $.compiler.version, $.compiler.commit_date, $.compiler.channel));
    core.set_color(Color::Cyan, Color::Black);

    print!("   .\"        )    ");
    core.set_color(Color::DarkGray, Color::Black);
    println!("{}", &build_info::format!("{}", $.timestamp));
    core.set_color(Color::Cyan, Color::Black);

    print!(",_/   ,    \\/\\_   ");
    core.set_color(Color::DarkGray, Color::Black);
    println!("{} {}", core.firmware_vendor(), &build_info::format!("{}, {}-bit {} Endian", $.target.cpu.arch,
        $.target.cpu.pointer_width, $.target.cpu.endianness));
    core.set_color(Color::Cyan, Color::Black);

    print!("\\_|    )_-\\ \\_-`  ");
    core.set_color(Color::DarkGray, Color::Black);
    println!("Firmware {}, HAL {}", core.firmware_revision(), core.uefi_revision());
    core.set_color(Color::Cyan, Color::Black);

    println!("   `-----` `--`   ");

    core.set_color(Color::LightGray, Color::Black);

    core.execute_kmode_binary("/bin/init", true);
    panic!("Attempted to kill init.");
}
