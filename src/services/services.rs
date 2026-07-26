use std::fs;
use std::fs::File;
use std::io::Write;

#[derive(Clone, PartialEq)]
pub struct AppFile {
    pub path: String,
    pub size: u64,
    pub is_dir: bool
}

pub struct FileServices;

impl FileServices {
    pub fn get_files_and_folders() -> Vec<AppFile> {
        let mut files: Vec<AppFile> = Vec::new();
        if let Ok(entries) = fs::read_dir(".") {
            for entry in entries.flatten() {
                if entry.file_type().unwrap().is_dir() {
                    files.push(AppFile::new(entry.path().to_str().unwrap().to_string(), entry.metadata().unwrap().len(), true));
                } else {
                    files.push(AppFile::new(entry.path().to_str().unwrap().to_string(), entry.metadata().unwrap().len(), false));
                }
            }
        }
        files
    }
}

impl AppFile {
    pub fn new(path: String, size: u64, is_dir: bool) -> Self {
        Self {
            path,
            size,
            is_dir
        }
    }

    pub fn file_name(self) -> String {
        self.path.split("/").last().unwrap().to_string()
    }

    pub fn read(self) -> Result<String, String> {
        let file = fs::read_to_string(self.path);
        if let Ok(result) = file { Ok(result) } else { Err(String::from("File not found")) }
    }

    pub fn save(self, content: &String) -> Result<String, String> {
        let file = File::create(self.path);
        if let Ok(mut file) = file {
            let result =file.write_all(content.as_bytes());
            if let Ok(_) = result {
                return Ok(String::from("Successfully saved"));
            }
            return Err(String::from("Error saving file"));
        }
        Err(String::from("File opening found"))
    }
}



