use std::fs;
use std::path::PathBuf;
use mkrelf::pe_to_elf;

fn place(source: &str, target: &str) {
    println!("{source}");

    if PathBuf::from(target).exists() {
        fs::remove_file(target).unwrap();
    }

    if !PathBuf::from(source).exists() {
        panic!("error: File {source} not found.");
    }

    fs::copy(source, target).unwrap();
}

fn dir(name: &str) {
    if PathBuf::from(name).exists() {
        return;
    }

    fs::create_dir_all(name).unwrap();
}

fn create_bundle(source: &str, directory: &str, destination: &str, ctx: u32) {
    dir(directory);
    place(source, &format!("{destination}"));
    pe_to_elf(&format!("{destination}"), ctx);
}

fn include_program(source: &str, destination: &str) {
    create_bundle(
        &format!("./os/target/x86_64-unknown-uefi/debug/{source}.efi"),
        "./esp/rootfs/System/Programs",
        &format!("./esp/rootfs/System/Programs/{destination}"),
        2
    );
}

fn main() {
    println!("mkrimg - Generate a working Russet system image from compiled files");

    dir("./esp/efi/boot");
    place("./os/target/x86_64-unknown-uefi/debug/rouse.efi", "./esp/efi/boot/bootx64.efi");

    create_bundle(
        "./os/target/x86_64-unknown-uefi/debug/velm.efi",
        "./esp/rootfs/System",
        "./esp/rootfs/System/Kernel",
        1
    );

    create_bundle(
        "./os/target/x86_64-unknown-uefi/debug/sable.efi",
        "./esp/rootfs/System",
        "./esp/rootfs/System/Init",
        1
    );

    include_program("demo", "DemoProgram");
    include_program("command-interpreter", "CommandInterpreter");
}
