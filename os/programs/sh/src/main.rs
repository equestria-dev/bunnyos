#![no_main]
#![no_std]

use alloc::string::{String, ToString};
use alloc::vec;
use uefi::prelude::*;
use uefi::{print, println, CStr16};
use uefi::fs::PathBuf;
use bunnyos_common::{transfer_system_table, CoreServices, ExecBinaryError};
use bunnyos_common::parser::Command;

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

    if !core.fs.file_exists("\\bunny\\root") {
        core.fs.mkdir("\\bunny\\root");
    }

    core.fs.chdir("\\bunny\\root").expect("Failed to switch to /root");

    loop {
        let pwd = core.fs.get_cwd();
        print!("\r{pwd}$ ");

        let cmd_str = &core.readline();
        if cmd_str.trim() == "" {
            continue;
        }

        let cmd = Command::build(cmd_str.trim());

        match cmd {
            Err(_) => println!("sh: invalid command"),
            Ok(cmd) => match cmd.command.as_str() {
                "pwd" => {
                    println!("{pwd}");
                },
                "exit" => {
                    return Status::SUCCESS;
                },
                "echo" => {
                    println!("{}", cmd.names.join(" "));
                },
                "cd" => {
                    if cmd.names.len() == 1 {
                        let mut path = core.fs.resolve_path(&cmd.names[0]);
                        if path == "" || !path.starts_with("\\bunny\\") {
                            path = String::from("\\bunny");
                        }

                        if core.fs.file_exists(&path) {
                            if core.fs.is_dir(&path) {
                                core.fs.chdir(&path).unwrap();
                            } else {
                                println!("cd: Not a directory: {}", cmd.names[0]);
                            }
                        } else {
                            println!("cd: No such file or directory: {}", cmd.names[0]);
                        }
                    } else if cmd.names.len() == 0 {
                        if let Err(_) = core.fs.chdir("\\bunny\\root") {
                            println!("cd: No such file or directory: /root");
                        }
                    } else {
                        println!("cd: Too many arguments");
                    }
                },
                _ => {
                    core.set_shared_variable("argv", cmd.to_bytes().as_slice()).unwrap();
                    let mut path: PathBuf = PathBuf::from(cstr16!("/bunny/bin"));

                    if cmd.command.starts_with("/") && cmd.command.len() > 1 {
                        let mut buf = vec![0; cmd.command.len() + 1];
                        path = PathBuf::from(cstr16!("/bunny"));
                        path.push(PathBuf::from(CStr16::from_str_with_buf(&cmd.command[1..], &mut buf).unwrap()));
                    } else if cmd.command.starts_with("./") && cmd.command.len() > 2 {
                        let mut buf = vec![0; core.fs.get_real_cwd().len() + 1];
                        path = PathBuf::from(CStr16::from_str_with_buf(&core.fs.get_real_cwd(), &mut buf).unwrap());

                        let mut buf = vec![0; cmd.command.len() + 1];
                        path.push(PathBuf::from(CStr16::from_str_with_buf(&cmd.command, &mut buf).unwrap()));
                    } else {
                        let mut buf = vec![0; cmd.command.len() + 1];
                        path.push(PathBuf::from(CStr16::from_str_with_buf(&cmd.command, &mut buf).unwrap()));
                    }

                    match core.execute_user_binary(&path.to_string()) {
                        Ok(_) | Err(ExecBinaryError::Finished) => (),
                        Err(ExecBinaryError::Load(e)) => println!("sh: Input/output error: {:?}", e),
                        Err(ExecBinaryError::ReadFS(e)) => println!("sh: Input/output error: {:?}", e),
                        Err(ExecBinaryError::ReadIO(e)) => println!("sh: Input/output error: {:?}", e),
                        Err(ExecBinaryError::NotFound) => if (cmd.command.starts_with("/") && cmd.command.len() > 1) ||
                            (cmd.command.starts_with("./") && cmd.command.len() > 2) {
                            println!("sh: No such file or directory: {}", cmd.command)
                        } else {
                            println!("sh: Command not found: {}", cmd.command)
                        },
                        Err(ExecBinaryError::OutOfMemory) => println!("sh: Killed (out of memory): {}", cmd.command),
                        Err(ExecBinaryError::Runtime(e)) => println!("sh: Killed ({}): {}", e.status(), cmd.command),
                        Err(ExecBinaryError::Unsupported) => println!("sh: Exec format error: {}", cmd.command)
                    }

                    core.delete_shared_variable("argv").unwrap();
                }
            }
        }
    }
}
