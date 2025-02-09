#![no_main]
#![no_std]

use alloc::string::{String, ToString};
use alloc::vec;
use uefi::prelude::*;
use uefi::{print, println, CStr16};
use uefi::fs::PathBuf;
use russet_common::{status_to_text, CoreServices, ExecBinaryError};
use russet_common::parser::Command;

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

    if !core.fs.file_exists("\\rootfs\\User") {
        core.fs.mkdir("\\rootfs\\User");
    }

    core.fs.chdir("\\rootfs\\User").expect("Failed to switch to /User");

    loop {
        let pwd = core.fs.get_cwd();
        print!("\r\n{pwd}> ");

        let cmd_str = &core.readline();
        if cmd_str.trim() == "" {
            continue;
        }

        let cmd = Command::build(cmd_str.trim());

        match cmd {
            Err(_) => println!("Illegal command."),
            Ok(cmd) => match cmd.command.as_str() {
                "GetCurrentDirectory" => {
                    println!("{pwd}");
                },
                "Exit" => {
                    return Status::SUCCESS;
                },
                "_Crash" => {
                    core.execute_kmode_binary("/System/Kernel", true).expect("TODO: panic message");
                },
                "Print" => {
                    println!("{}", cmd.names.join(" "));
                },
                "ChangeDirectory" => {
                    if cmd.names.len() == 1 {
                        let mut path = core.fs.resolve_path(&cmd.names[0]);
                        if path == "" || !path.starts_with("\\rootfs\\") {
                            path = String::from("\\rootfs");
                        }

                        if core.fs.file_exists(&path) {
                            if core.fs.is_dir(&path) {
                                core.fs.chdir(&path).unwrap();
                            } else {
                                println!("The path \"{}\" is not a valid directory.", cmd.names[0]);
                            }
                        } else {
                            println!("The file \"{}\" could not be found.", cmd.names[0]);
                        }
                    } else if cmd.names.len() == 0 {
                        if let Err(_) = core.fs.chdir("\\rootfs\\User") {
                            println!("The file \"/User\" could not be found.");
                        }
                    } else {
                        println!("Invalid command use.");
                    }
                },
                "GetCommandFile" => {
                    for name in cmd.names {
                        let mut path: PathBuf = PathBuf::from(cstr16!("/rootfs/System/Programs"));

                        if name.starts_with("/") && name.len() > 1 {
                            let mut buf = vec![0; name.len() + 1];
                            path = PathBuf::from(cstr16!("/rootfs"));
                            path.push(PathBuf::from(CStr16::from_str_with_buf(&name[1..], &mut buf).unwrap()));
                        } else if name.starts_with("./") && name.len() > 2 {
                            let mut buf = vec![0; core.fs.get_real_cwd().len() + 1];
                            path = PathBuf::from(CStr16::from_str_with_buf(&core.fs.get_real_cwd(), &mut buf).unwrap());

                            let mut buf = vec![0; name.len() + 1];
                            path.push(PathBuf::from(CStr16::from_str_with_buf(&name, &mut buf).unwrap()));
                        } else {
                            let mut buf = vec![0; name.len() + 1];
                            path.push(PathBuf::from(CStr16::from_str_with_buf(&name, &mut buf).unwrap()));
                        }

                        if core.fs.file_exists(&path.to_string()) && core.fs.is_file(&path.to_string()) {
                            let path = path.to_string();
                            if path.starts_with("\\rootfs") {
                                let path = path[7..].replace("\\", "/");
                                println!("{}", if path.trim() == "" {
                                    String::from("/")
                                } else {
                                    path
                                });
                            } else {
                                println!("//?{}", path.replace("\\", "/"))
                            }
                        } else {
                            println!("The command \"{name}\" could not found.");
                        }
                    }
                }
                _ => {
                    core.set_shared_variable("argv", cmd.to_bytes().as_slice()).unwrap();
                    let mut path: PathBuf = PathBuf::from(cstr16!("/rootfs/System/Programs"));

                    if cmd.command.starts_with("/") && cmd.command.len() > 1 {
                        let mut buf = vec![0; cmd.command.len() + 1];
                        path = PathBuf::from(cstr16!("/rootfs"));
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
                        Err(ExecBinaryError::Load(e)) => println!("An internal system error has occurred while loading this program. {:?}", e),
                        Err(ExecBinaryError::ReadFS(e)) => println!("An internal system error has occurred while reading this program. {:?}", e),
                        Err(ExecBinaryError::ReadIO(e)) => println!("An internal system error has occurred while processing data from this program. {:?}", e),
                        Err(ExecBinaryError::NotFound) => if (cmd.command.starts_with("/") && cmd.command.len() > 1) ||
                            (cmd.command.starts_with("./") && cmd.command.len() > 2) {
                            println!("The file \"{}\" could not be found.", cmd.command)
                        } else {
                            println!("\"{}\" is not recognized as a valid internal command or external executable program. \
                            Please refer to the operating system manual for additional information.", cmd.command)
                        },
                        Err(ExecBinaryError::OutOfMemory) => println!("The system is low on memory and \"{}\" had to be stopped.", cmd.command),
                        Err(ExecBinaryError::Runtime(e)) => println!("The program \"{}\" has stopped working. ({})", cmd.command, status_to_text(e.status())),
                        Err(ExecBinaryError::Unsupported) => println!("\"{}\" is not a valid BunnyOS program.", cmd.command)
                    }

                    core.delete_shared_variable("argv").unwrap();
                }
            }
        }
    }
}
