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
pub const SUPPORTED_ABI: [u32; 1] = [2];
pub const DEFAULT_SHELL: &str = "/System/Programs/CommandInterpreter";
pub const DEFAULT_KERNEL: &str = "/System/Kernel";

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
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn init(value: SystemTable<Boot>, panic: bool) -> Self {
        FATAL_PANIC = panic;
        Self {
            fs: CoreFileSystem::from(value.unsafe_clone()),
            system_table: value,
        }
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn transfer_system_table(&mut self, h: Handle, fallback_build_info: String) {
        SYSTEM_TABLE = Some(self.get_system_table());
        HANDLE = Some(h);
        BUILD_INFO = Some(fallback_build_info);
        if let Ok((data, _)) = self.get_shared_variable("Russet.OSString") {
            if let Ok(string) = String::from_utf8(data) {
                BUILD_INFO = Some(string);
            }
        }
    }

    #[allow(clippy::missing_safety_doc)]
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

    pub fn get_shared_variable(&mut self, name: &str) -> Result<(Vec<u8>, VariableAttributes), Error> {
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

    pub fn delete_shared_variable(&mut self, name: &str) -> uefi::Result {
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
        let string = format!("\\rootfs{}", path.replace("/", "\\"));
        let mut buf: Vec<u16> = vec![0; string.len() + 1];
        let cstr16 = CStr16::from_str_with_buf(&string, &mut buf).unwrap();
        let path: CString16 = CString16::from(cstr16);
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
                                    panic!("CRITICAL_PROCESS_DIED")
                                } else {
                                    Err(ExecBinaryError::Finished)
                                }
                                Err(e) => {
                                    match e.status() {
                                        Status::UNSUPPORTED => if strict {
                                            panic!("CRITICAL_PROCESS_DIED")
                                        } else {
                                            Err(ExecBinaryError::Unsupported)
                                        },
                                        _ => if strict {
                                            panic!("CRITICAL_PROCESS_DIED (RUN: {:?})", e)
                                        } else {
                                            Err(ExecBinaryError::Runtime(e))
                                        }
                                    }
                                }
                            }
                        },
                        Err(e) => if strict {
                            panic!("CRITICAL_PROCESS_DIED (LOAD: {:?})", e)
                        } else {
                            match e.status() {
                                Status::UNSUPPORTED => Err(ExecBinaryError::Unsupported),
                                _ => Err(ExecBinaryError::Load(e))
                            }
                        }
                    }
                } else if strict {
                    panic!("BOUND_IMAGE_UNSUPPORTED")
                } else {
                    Err(ExecBinaryError::Unsupported)
                }
            },
            Err(e) => {
                match e {
                    Io(e) => {
                        match e.uefi_error.status() {
                            Status::NOT_FOUND => if strict {
                                panic!("FILE_INITIALIZATION_FAILED")
                            } else {
                                Err(ExecBinaryError::NotFound)
                            },
                            Status::OUT_OF_RESOURCES => if strict {
                                panic!("MEMORY_MANAGEMENT")
                            } else {
                                Err(ExecBinaryError::OutOfMemory)
                            },
                            _ => if strict {
                                panic!("CRITICAL_PROCESS_DIED (READ: {:?})", e)
                            } else {
                                Err(ExecBinaryError::ReadIO(e))
                            }
                        }
                    },
                    _ => if strict {
                        panic!("CRITICAL_PROCESS_DIED (LOAD: {:?})", e)
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
        let cstr16 = CStr16::from_str_with_buf(path, &mut buf).unwrap();
        let path: CString16 = CString16::from(cstr16);
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
                if version.n_type == 1 && version.name == "Russet " {
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
                    let calculated_checksum = crc.checksum(data.0);

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

#[derive(Debug)]
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

pub fn status_to_text<'a>(status: Status) -> &'a str {
    match status {
        Status::SUCCESS => "The operation completed successfully.",
        Status::WARN_UNKNOWN_GLYPH => "The program used a character that could not be rendered.",
        Status::WARN_DELETE_FAILURE => "The program closed a file handle that could not be deleted.",
        Status::WARN_WRITE_FAILURE => "The program closed a file handle that could not be written.",
        Status::WARN_BUFFER_TOO_SMALL => "The program encountered a buffer underflow.",
        Status::WARN_STALE_DATA => "The program contained data that was not updated within the required time.",
        Status::WARN_FILE_SYSTEM => "The program contained data that was a compatible filesystem.",
        Status::WARN_RESET_REQUIRED => "The program performed an operation that requires a system restart.",
        Status::LOAD_ERROR => "The image failed to load.",
        Status::INVALID_PARAMETER => "A parameter was incorrect.",
        Status::UNSUPPORTED => "The requested operation is not supported on this system.",
        Status::BAD_BUFFER_SIZE => "The supplied buffer was of improper size for the request.",
        Status::BUFFER_TOO_SMALL => "The supplied buffer was not large enough to hold the requested data.",
        Status::NOT_READY => "The device is not ready.",
        Status::DEVICE_ERROR => "The device reported a hardware error while attempting the operation.",
        Status::WRITE_PROTECTED => "The device cannot be written to.",
        Status::OUT_OF_RESOURCES => "The system has run out of resources.",
        Status::VOLUME_CORRUPTED => "The file system is corrupted.",
        Status::VOLUME_FULL => "The file system is full.",
        Status::NO_MEDIA => "The device does not contain valid media.",
        Status::MEDIA_CHANGED => "The device has changed media since the last access.",
        Status::NOT_FOUND => "The requested resource could not be found.",
        Status::ACCESS_DENIED => "Access is denied.",
        Status::NO_RESPONSE => "The remote server did not respond to the request.",
        Status::NO_MAPPING => "No route to the requested device exists.",
        Status::TIMEOUT => "The request did not complete in a timely manner.",
        Status::NOT_STARTED => "The program requested a protocol that has not been initialized.",
        Status::ALREADY_STARTED => "The program initialized a protocol that was already initialized.",
        Status::ABORTED => "The operation was aborted abruptly.",
        Status::ICMP_ERROR => "The network connection encountered an ICMP protocol error.",
        Status::TFTP_ERROR => "The network connection encountered a TFTP protocol error.",
        Status::PROTOCOL_ERROR => "The network connection encountered a protocol error.",
        Status::INCOMPATIBLE_VERSION => "The function is not compatible with the requested version.",
        Status::SECURITY_VIOLATION => "The operation constitutes a violation of the security policy.",
        Status::CRC_ERROR => "The operation failed a consistency check.",
        Status::END_OF_MEDIA => "The device has no more data to provide.",
        Status::END_OF_FILE => "The file has no more data to provide.",
        Status::INVALID_LANGUAGE => "The requested language is invalid.",
        Status::COMPROMISED_DATA => "The data has not been securely validated and could be compromised.",
        Status::IP_ADDRESS_CONFLICT => "The network address allocation has led to a conflict.",
        Status::HTTP_ERROR => "The network connection encountered an HTTP protocol error.",
        _ => "An unknown system error has occurred."
    }
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
