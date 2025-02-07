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

fn main() {
    println!("mkrimg - Generate a working Russet system image from compiled files");

    dir("./esp/efi/boot");
    place("./os/target/x86_64-unknown-uefi/debug/rouse.efi", "./esp/efi/boot/bootx64.efi");

    dir("./esp/rootfs/System");
    place("./os/target/x86_64-unknown-uefi/debug/velm.efi", "./esp/rootfs/System/Kernel");
    pe_to_elf("./esp/rootfs/System/Kernel", 1);

    dir("./esp/rootfs/System/Programs");
    place("./os/target/x86_64-unknown-uefi/debug/sable.efi", "./esp/rootfs/System/Init");
    pe_to_elf("./esp/rootfs/System/Init", 1);

    for program in fs::read_dir("./os/programs").unwrap() {
        let entry = program.unwrap();
        let name_os = entry.file_name();
        let name = name_os.to_str().unwrap();

        place(&format!("./os/target/x86_64-unknown-uefi/debug/{name}.efi"),
        &format!("./esp/rootfs/System/Programs/{name}"));
        pe_to_elf(&format!("./esp/rootfs/System/Programs/{name}"), 2);
    }
}
