use alloc::{format, vec};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use uefi::CStr16;
use uefi::fs::{FileSystem, Path, UefiDirectoryIter};
use uefi::table::{Boot, SystemTable};

pub struct CoreFileSystem {
    pub(crate) cwd: String,
    system_table: SystemTable<Boot>
}

impl From<SystemTable<Boot>> for CoreFileSystem {
    fn from(value: SystemTable<Boot>) -> Self {
        Self {
            cwd: String::from("\\bunny"),
            system_table: value
        }
    }
}

impl CoreFileSystem {
    fn get_fs(&self) -> FileSystem {
        let handle = self.system_table.boot_services().image_handle();
        let fs = self.system_table.boot_services().get_image_file_system(handle).expect("Failed to start up filesystem");
        FileSystem::new(fs)
    }

    pub fn get_cwd(&self) -> String {
        if self.cwd.starts_with("\\bunny") {
            let cwd = self.cwd[6..].replace("\\", "/");
            if cwd.trim() == "" {
                String::from("/")
            } else {
                cwd
            }
        } else {
            format!("//?{}", self.cwd[6..].replace("\\", "/"))
        }
    }

    pub fn chdir(&mut self, dir: &str) -> Result<(), ()> {
        if dir == "." {
            return Ok(());
        }

        if !self.file_exists(dir) {
            Err(())
        } else {
            self.cwd = self.resolve_path(dir);

            if !self.cwd.starts_with("\\bunny\\") {
                self.cwd = String::from("\\bunny");
            }

            Ok(())
        }
    }

    //noinspection RsSelfConvention
    pub fn is_dir(&mut self, path: &str) -> bool {
        if path.is_empty() {
            return true;
        }
        let mut buf: Vec<u16> = vec![0; path.len() + 1];
        match self.get_fs().metadata(Path::new(&CStr16::from_str_with_buf(path, &mut buf).unwrap())) {
            Ok(b) => b.is_directory(),
            Err(_) => false
        }
    }

    pub fn file_exists(&mut self, path: &str) -> bool {
        if path.is_empty() {
            return true;
        }
        let mut buf: Vec<u16> = vec![0; path.len() + 1];
        self.get_fs().try_exists(Path::new(&CStr16::from_str_with_buf(path, &mut buf).unwrap())).unwrap_or(false)
    }

    pub fn mkdir(&mut self, path: &str) {
        let mut buf: Vec<u16> = vec![0; path.len() + 1];
        self.get_fs().create_dir_all(Path::new(&CStr16::from_str_with_buf(path, &mut buf).unwrap())).unwrap();
    }

    pub fn get_real_cwd(&self) -> String {
        self.cwd.clone()
    }

    pub fn resolve_path(&self, orig_og_path: &str) -> String {
        let og_path = orig_og_path.replace('\\', "/");
        let mut final_path = self.cwd.to_string().replace('/', "\\");
        let path = &og_path.replace('/', "\\");
        let mut buf = vec![0; path.len() + 1];
        let cstr = CStr16::from_str_with_buf(path, &mut buf).unwrap();
        let path = Path::new(cstr);

        if og_path.starts_with('/') {
            final_path = String::from("");
        }

        for i in path.components() {
            if i.to_string() == ".." {
                let parts = final_path.split('\\').collect::<Vec<&str>>();
                let len = parts.len() - 1;

                final_path = parts[..len].join("\\");
            } else if i.to_string() != "." {
                final_path = format!("{}\\{}", final_path, i).replace('/', "\\");
            }
        }

        final_path.to_string().replace("\\\\", "\\")
    }

    pub fn rename(&mut self, old: &str, new: &str) {
        let mut buf: Vec<u16> = vec![0; old.len() + 1];
        let mut buf2: Vec<u16> = vec![0; new.len() + 1];
        self.get_fs().rename(Path::new(&CStr16::from_str_with_buf(old, &mut buf).unwrap()),
                          Path::new(&CStr16::from_str_with_buf(new, &mut buf2).unwrap())).expect("Failed to move");
    }

    pub fn copy_file(&mut self, old: &str, new: &str) {
        let mut buf: Vec<u16> = vec![0; old.len() + 1];
        let mut buf2: Vec<u16> = vec![0; new.len() + 1];
        self.get_fs().copy(Path::new(&CStr16::from_str_with_buf(old, &mut buf).unwrap()),
                        Path::new(&CStr16::from_str_with_buf(new, &mut buf2).unwrap())).expect("Failed to move");
    }

    pub fn is_file(&self, path: &str) -> bool {
        if path.is_empty() {
            return true;
        }
        let mut buf: Vec<u16> = vec![0; path.len() + 1];
        match self.get_fs().metadata(Path::new(&CStr16::from_str_with_buf(path, &mut buf).unwrap())) {
            Ok(b) => b.is_regular_file(),
            Err(_) => false
        }
    }

    pub fn read_file(&self, path: &str) -> Option<String> {
        if path.is_empty() {
            return None;
        }
        let mut buf: Vec<u16> = vec![0; path.len() + 1];
        match self.get_fs().read_to_string(Path::new(&CStr16::from_str_with_buf(path, &mut buf).unwrap())) {
            Ok(b) => Some(b),
            Err(_) => None
        }
    }

    pub fn write_file(&self, path: &str, text: &str) {
        if path.is_empty() {
            return;
        }
        let mut buf: Vec<u16> = vec![0; path.len() + 1];
        self.get_fs().write(Path::new(&CStr16::from_str_with_buf(path, &mut buf).unwrap()), text.as_bytes()).unwrap();
    }

    pub fn scandir(&self, path: &str) -> UefiDirectoryIter {
        let mut buf: Vec<u16> = vec![0; path.len() + 1];
        match self.get_fs().read_dir(Path::new(&CStr16::from_str_with_buf(path, &mut buf).unwrap())) {
            Ok(b) => b,
            Err(_) => panic!("Failed to scan directory {}", path)
        }
    }

    pub fn rmdir(&mut self, path: &str) {
        let mut buf: Vec<u16> = vec![0; path.len() + 1];
        self.get_fs().remove_dir(Path::new(&CStr16::from_str_with_buf(path, &mut buf).unwrap())).unwrap();
    }

    pub fn unlink(&mut self, path: &str) {
        let mut buf: Vec<u16> = vec![0; path.len() + 1];
        self.get_fs().remove_file(Path::new(&CStr16::from_str_with_buf(path, &mut buf).unwrap())).unwrap();
    }

    pub fn recursive_rmdir(&mut self, path: &str) {
        let mut buf: Vec<u16> = vec![0; path.len() + 1];
        self.get_fs().remove_dir_all(Path::new(&CStr16::from_str_with_buf(path, &mut buf).unwrap())).unwrap();
    }
}
