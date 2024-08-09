use std::fs;
use std::path::PathBuf;

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
    println!("mkbimg - Generate a working BunnyOS system image from compiled files");

    dir("./esp/efi/boot");
    place("./os/target/x86_64-unknown-uefi/debug/bunnyloader.efi", "./esp/efi/boot/bootx64.efi");

    dir("./esp/bunny/boot");
    place("./os/target/x86_64-unknown-uefi/debug/bunnycore.efi", "./esp/bunny/boot/kernel");

    dir("./esp/bunny/bin");
    place("./os/target/x86_64-unknown-uefi/debug/rabbinit.efi", "./esp/bunny/bin/init");

    for program in fs::read_dir("./os/programs").unwrap() {
        let entry = program.unwrap();
        let name_os = entry.file_name();
        let name = name_os.to_str().unwrap();

        place(&format!("./os/target/x86_64-unknown-uefi/debug/{name}.efi"),
        &format!("./esp/bunny/bin/{name}"));
    }
}
