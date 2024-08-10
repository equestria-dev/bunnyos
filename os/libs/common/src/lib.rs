#![no_std]
extern crate alloc;

use alloc::{format, vec};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use uefi::proto::console::text::{Color, Key};
use uefi::prelude::*;
use uefi::{guid, print, CStr16, CString16, Char16, Error, Guid};
use uefi::fs::{FileSystemResult, IoError};
use uefi::fs::Error::Io;
use uefi::proto::device_path::LoadedImageDevicePath;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::boot::{LoadImageSource, ScopedProtocol};
use uefi::table::runtime::{VariableAttributes, VariableVendor};
use crate::fs::CoreFileSystem;

use core::panic::PanicInfo;
use elf::{ElfBytes, ParseError};
use elf::endian::AnyEndian;
use elf::note::Note;
use uefi::proto::console::text::Output;
use uefi::println;

pub mod parser;
mod fs;

static mut SYSTEM_TABLE: Option<SystemTable<Boot>> = None;
static mut HANDLE: Option<Handle> = None;
static mut BUILD_INFO: Option<String> = None;
static mut FATAL_PANIC: bool = false;

pub const GUID: Guid = guid!("cf3dd8e5-823e-4d06-8caf-d0fd9e49f588");
pub const VENDOR: VariableVendor = VariableVendor(GUID);
pub const OS_VERSION: &str = "0.1";
pub const SUPPORTED_ABI: [u32; 1] = [1];
pub const DEFAULT_SHELL: &str = "/bin/sh";
pub const DEFAULT_KERNEL: &str = "/boot/kernel";

pub struct CoreServices {
    system_table: SystemTable<Boot>,
    pub fs: CoreFileSystem
}

#[panic_handler]
#[allow(unused_must_use, static_mut_refs)]
unsafe fn panic(info: &PanicInfo) -> ! {
    if !FATAL_PANIC {
        println!("{}", info);
        if let (Some(ref mut st), Some(ref mut h)) = (&mut SYSTEM_TABLE, &mut HANDLE) {
            let mut return_data = Char16::try_from(' ').unwrap();
            st.boot_services().exit(*h, Status::ABORTED, 0, &mut return_data);
        }
        loop {}
    }

    if let Some(ref mut st) = &mut SYSTEM_TABLE {
        let stdout: &mut Output = st.stdout();
        stdout.set_color(Color::White, Color::Red);
        stdout.enable_cursor(false);
        stdout.clear();
    }

    println!("*** STOP: {}", info.location().unwrap().to_string().replace("\\", "/"));
    println!("{}", info.message());

    if let Some(ref mut build) = &mut BUILD_INFO {
        println!("\n{}", build);
    }

    if let Some(ref mut st) = &mut SYSTEM_TABLE {
        println!("\nFirmware: {} {}", st.firmware_vendor(), st.firmware_revision());
        println!("Specification: {}", st.uefi_revision());
    }

    println!("\nPlease restart the system.");
    loop {}
}

impl CoreServices {
    pub unsafe fn init(value: SystemTable<Boot>, panic: bool) -> Self {
        FATAL_PANIC = panic;
        Self {
            fs: CoreFileSystem::from(value.unsafe_clone()),
            system_table: value,
        }
    }

    pub unsafe fn get_system_table(&self) -> SystemTable<Boot> {
        self.system_table.unsafe_clone()
    }

    pub fn set_shared_variable(&mut self, name: &str, value: &[u8]) -> uefi::Result {
        let mut buf = vec![0; name.len() + 1];

        self.system_table.runtime_services().set_variable(
            CStr16::from_str_with_buf(name, &mut buf).unwrap_or(cstr16!("")),
            &VENDOR,
            VariableAttributes::BOOTSERVICE_ACCESS,
            value
        )
    }

    pub fn get_shared_variable<'a>(&mut self, name: &str) -> Result<(Vec<u8>, VariableAttributes), Error> {
        let mut buf1 = vec![0; name.len() + 1];
        let mut buf2 = [0u8; 65536];

        match self.system_table.runtime_services().get_variable(
            CStr16::from_str_with_buf(name, &mut buf1).unwrap_or(cstr16!("")),
            &VENDOR,
            &mut buf2
        ) {
            Ok(d) => Ok((d.0.to_vec(), d.1)),
            Err(e) => Err(e)
        }
    }

    pub fn delete_shared_variable<'a>(&mut self, name: &str) -> uefi::Result {
        let mut buf = vec![0; name.len() + 1];
        self.system_table.runtime_services().delete_variable(
            CStr16::from_str_with_buf(name, &mut buf).unwrap_or(cstr16!("")),
            &VENDOR
        )
    }

    pub fn set_color(&mut self, fg: Color, bg: Color) -> uefi::Result<()> {
        self.system_table.stdout().set_color(fg, bg)
    }

    pub fn readline(&mut self) -> String {
        let system_table = &mut self.system_table;

        let mut out: String = String::from("");
        let mut chars: u32 = 0;

        loop {
            let mut events = [system_table.stdin().wait_for_key_event().unwrap()];
            system_table.boot_services()
                .wait_for_event(&mut events)
                .discard_errdata().expect("Failed to discard errors");

            let ret = Char16::try_from('\r').unwrap();
            let bks = Char16::try_from('\x08').unwrap();
            let ctc = Char16::try_from('\u{3}').unwrap();
            match system_table.stdin().read_key().expect("Failed to read key") {
                Some(Key::Printable(key)) if key == ret => {
                    print!("\r\n");
                    return out;
                }

                Some(Key::Printable(key)) if key == bks => {
                    if chars > 0 {
                        chars -= 1;
                        out = String::from(&out[..out.len() - 1]);
                        print!("\x08");
                    }
                }

                Some(Key::Printable(key)) if key == ctc => {
                    print!("\r\n");
                    return String::from("");
                }

                Some(Key::Printable(key)) => {
                    chars += 1;
                    out += &key.to_string();
                    print!("{}", &key.to_string());
                }

                _ => {}
            }
        }
    }

    fn get_kernel_binary(&self, path: &str) -> FileSystemResult<Vec<u8>> {
        let boot_services = self.system_table.boot_services();
        let string = format!("\\bunny{}", path.replace("/", "\\"));
        let mut buf: Vec<u16> = vec![0; string.len() + 1];
        let cstr16 = CStr16::from_str_with_buf(&string, &mut buf).unwrap();
        let path: CString16 = CString16::try_from(cstr16).unwrap();
        let fs: ScopedProtocol<SimpleFileSystem> = boot_services.get_image_file_system(boot_services.image_handle()).unwrap();
        let mut fs = uefi::fs::FileSystem::new(fs);
        fs.read(path.as_ref())
    }

    pub fn execute_kmode_binary(&self, path: &str, strict: bool) -> Result<(), ExecBinaryError> {
        let boot_services = self.system_table.boot_services();

        let loaded_image = boot_services
            .open_protocol_exclusive::<LoadedImageDevicePath>(boot_services.image_handle())
            .unwrap();

        let binary = self.get_kernel_binary(path);

        match binary {
            Ok(data) => {
                if let Ok(data) = self.elf_to_pe(data.as_slice(), ElfContext::Kernel) {
                    match boot_services.load_image(boot_services.image_handle(), LoadImageSource::FromBuffer {
                        buffer: data.as_slice(),
                        file_path: Some(&**loaded_image)
                    }) {
                        Ok(handle) => {
                            match boot_services.start_image(handle) {
                                Ok(_) => if strict {
                                    panic!("Attempted to kill!")
                                } else {
                                    Err(ExecBinaryError::Finished)
                                }
                                Err(e) => {
                                    match e.status() {
                                        Status::UNSUPPORTED => if strict {
                                            panic!("Attempted to kill!")
                                        } else {
                                            Err(ExecBinaryError::Unsupported)
                                        },
                                        _ => if strict {
                                            panic!("Attempted to kill! - Run error: {:?}", e)
                                        } else {
                                            Err(ExecBinaryError::Runtime(e))
                                        }
                                    }
                                }
                            }
                        },
                        Err(e) => if strict {
                            panic!("Attempted to kill! - Load error: {:?}", e)
                        } else {
                            match e.status() {
                                Status::UNSUPPORTED => Err(ExecBinaryError::Unsupported),
                                _ => Err(ExecBinaryError::Load(e))
                            }
                        }
                    }
                } else {
                    if strict {
                        panic!("Invalid executable")
                    } else {
                        Err(ExecBinaryError::Unsupported)
                    }
                }
            },
            Err(e) => {
                match e {
                    Io(e) => {
                        match e.uefi_error.status() {
                            Status::NOT_FOUND => if strict {
                                panic!("Attempted to kill! - Not found")
                            } else {
                                Err(ExecBinaryError::NotFound)
                            },
                            Status::OUT_OF_RESOURCES => if strict {
                                panic!("Out of memory!")
                            } else {
                                Err(ExecBinaryError::OutOfMemory)
                            },
                            _ => if strict {
                                panic!("Attempted to kill! - Read error: {:?}", e)
                            } else {
                                Err(ExecBinaryError::ReadIO(e))
                            }
                        }
                    },
                    _ => if strict {
                        panic!("Attempted to kill! - Read error: {:?}", e)
                    } else {
                        Err(ExecBinaryError::ReadFS(e))
                    }
                }
            }
        }
    }

    fn get_user_binary(&self, path: &str) -> FileSystemResult<Vec<u8>> {
        let boot_services = self.system_table.boot_services();
        let mut buf: Vec<u16> = vec![0; path.len() + 1];
        let cstr16 = CStr16::from_str_with_buf(&path, &mut buf).unwrap();
        let path: CString16 = CString16::try_from(cstr16).unwrap();
        let fs: ScopedProtocol<SimpleFileSystem> = boot_services.get_image_file_system(boot_services.image_handle()).unwrap();
        let mut fs = uefi::fs::FileSystem::new(fs);
        fs.read(path.as_ref())
    }

    pub fn execute_user_binary(&self, path: &str) -> Result<(), ExecBinaryError> {
        let boot_services = self.system_table.boot_services();

        let loaded_image = boot_services
            .open_protocol_exclusive::<LoadedImageDevicePath>(boot_services.image_handle())
            .unwrap();

        let binary = self.get_user_binary(path);

        match binary {
            Ok(data) => {
                if let Ok(data) = self.elf_to_pe(data.as_slice(), ElfContext::User) {
                    match boot_services.load_image(boot_services.image_handle(), LoadImageSource::FromBuffer {
                        buffer: data.as_slice(),
                        file_path: Some(&**loaded_image)
                    }) {
                        Ok(handle) => {
                            match boot_services.start_image(handle) {
                                Ok(_) => Err(ExecBinaryError::Finished),
                                Err(e) => {
                                    match e.status() {
                                        Status::UNSUPPORTED => Err(ExecBinaryError::Unsupported),
                                        _ => Err(ExecBinaryError::Runtime(e))
                                    }
                                }
                            }
                        },
                        Err(e) => match e.status() {
                            Status::UNSUPPORTED => Err(ExecBinaryError::Unsupported),
                            _ => Err(ExecBinaryError::Load(e))
                        }
                    }
                } else {
                    Err(ExecBinaryError::Unsupported)
                }
            },
            Err(e) => {
                match e {
                    Io(e) => {
                        match e.uefi_error.status() {
                            Status::NOT_FOUND => Err(ExecBinaryError::NotFound),
                            Status::OUT_OF_RESOURCES => Err(ExecBinaryError::OutOfMemory),
                            _ => Err(ExecBinaryError::ReadIO(e))
                        }
                    },
                    _ => Err(ExecBinaryError::ReadFS(e))
                }
            }
        }
    }

    pub fn firmware_vendor(&self) -> String {
        self.system_table.firmware_vendor().to_string()
    }

    pub fn uefi_revision(&self) -> String {
        self.system_table.uefi_revision().to_string()
    }

    pub fn firmware_revision(&self) -> u32 {
        self.system_table.firmware_revision()
    }

    pub fn elf_to_pe(&self, elf: &[u8], expected_context: ElfContext) -> Result<Vec<u8>, ElfError> {
        let expected_context = expected_context as u32;
        let file = ElfBytes::<AnyEndian>::minimal_parse(elf)?;

        let version_header = file.section_header_by_name(".note.tag")?.ok_or(ElfError::SectionNotFound)?;
        match file.section_data_as_notes(&version_header)?.next().ok_or(ElfError::SectionNotFound)? {
            Note::GnuAbiTag(_) => Err(ElfError::InvalidPlatform),
            Note::GnuBuildId(_) => Err(ElfError::InvalidPlatform),
            Note::Unknown(version) => {
                if version.n_type == 1 && version.name == "BunnyOS" {
                    let data = version.desc;

                    let context_bytes = [data[4], data[5], data[6], data[7]];
                    let context = u32::from_le_bytes(context_bytes);

                    let abi_version_bytes = [data[8], data[9], data[10], data[11]];
                    let abi_version = u32::from_le_bytes(abi_version_bytes);

                    let checksum_bytes = [data[12], data[13], data[14], data[15]];
                    let checksum = u32::from_le_bytes(checksum_bytes);

                    let source_header = file.section_header_by_name(".text")?.ok_or(ElfError::SectionNotFound)?;
                    let data = file.section_data(&source_header)?;

                    let crc: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_BZIP2);
                    let calculated_checksum = crc.checksum(&data.0);

                    if context != expected_context {
                        Err(ElfError::InvalidContext)
                    } else if !SUPPORTED_ABI.contains(&abi_version) {
                        Err(ElfError::UnsupportedABI)
                    } else if calculated_checksum != checksum {
                        Err(ElfError::Corrupted)
                    } else {
                        Ok(Vec::from(data.0))
                    }
                } else {
                    Err(ElfError::InvalidPlatform)
                }
            }
        }
    }
}

pub enum ElfError {
    Parse(ParseError),
    SectionNotFound,
    InvalidPlatform,
    InvalidContext,
    UnsupportedABI,
    Corrupted
}

impl From<ParseError> for ElfError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}

pub unsafe fn transfer_system_table(st: SystemTable<Boot>, h: Handle, build_info: String) {
    SYSTEM_TABLE = Some(st);
    HANDLE = Some(h);
    BUILD_INFO = Some(build_info);
}

#[derive(Debug)]
pub enum ExecBinaryError {
    Finished,
    Unsupported,
    OutOfMemory,
    NotFound,
    Runtime(Error),
    Load(Error),
    ReadIO(IoError),
    ReadFS(uefi::fs::Error),
}

#[repr(u32)]
#[derive(Debug)]
pub enum ElfContext {
    Kernel = 1,
    User = 2
}
