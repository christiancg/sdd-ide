use tokio::io::AsyncWriteExt;

#[derive(Clone, PartialEq)]
pub struct AppFile {
    pub path: String,
    pub size: u64,
    pub is_dir: bool
}

pub struct FileServices;

impl FileServices {
    pub async fn get_files_and_folders() -> Vec<AppFile> {
        let mut files: Vec<AppFile> = Vec::new();
        if let Ok(mut entries) = tokio::fs::read_dir(".").await {
            while let Some(entry) = entries.next_entry().await.unwrap() {
                if entry.file_type().await.unwrap().is_dir() {
                    files.push(AppFile::new(entry.path().to_str().unwrap().to_string(), entry.metadata().await.unwrap().len(), true));
                } else {
                    files.push(AppFile::new(entry.path().to_str().unwrap().to_string(), entry.metadata().await.unwrap().len(), false));
                }
            }
        }
        files
    }

    pub async fn create_new_folder(path: String, name: String) -> Result<AppFile, std::io::Error> {
        let full_path = format!("{}/{}", path, name);
        let created = tokio::fs::create_dir_all(full_path).await;
        if created.is_ok() {
            Ok(AppFile::new(path.clone(), 0, true))
        } else {
            Err(created.err().unwrap())
        }
    }

    pub async fn create_new_file(path: String, file_name: String) -> Result<AppFile, std::io::Error> {
        let complete_path = path.clone() + "/" + &*file_name;
        let created = tokio::fs::File::create(complete_path.clone()).await;
        if created.is_ok() {
            Ok(AppFile::new(complete_path.clone(), 0, false))
        } else {
            Err(created.err().unwrap())
        }
    }

    pub async fn delete(path: String, is_dir: bool) -> Result<(), std::io::Error> {
        let result: Result<(), std::io::Error>;
        if is_dir {
            result = tokio::fs::remove_dir_all(path.clone()).await;
        } else {
            result = tokio::fs::remove_file(path.clone()).await;
        }
        if result.is_ok() {
            Ok(())
        } else {
            Err(result.err().unwrap())
        }
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

    pub async fn read(self) -> Result<String, String> {
        let file = tokio::fs::read_to_string(self.path).await;
        if let Ok(result) = file { Ok(result) } else { Err(String::from("File not found")) }
    }

    pub async fn save(self, content: &String) -> Result<String, String> {
        let file = tokio::fs::File::create(self.path).await;
        if let Ok(mut file) = file {
            let result = file.write_all(content.as_bytes()).await;
            if let Ok(_) = result {
                return Ok(String::from("Successfully saved"));
            }
            return Err(String::from("Error saving file"));
        }
        Err(String::from("File opening found"))
    }
}



