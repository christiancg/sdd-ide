use std::pin::Pin;
use tokio::io::AsyncWriteExt;

#[derive(Clone, PartialEq)]
pub struct AppFile {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_hidden: bool,
    pub children: Option<Vec<AppFile>>,
}

pub struct FileServices;

#[cfg(windows)]
fn is_hidden_entry(_file_name: &str, metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
}

#[cfg(not(windows))]
fn is_hidden_entry(file_name: &str, _metadata: &std::fs::Metadata) -> bool {
    file_name.starts_with('.')
}

impl FileServices {
    pub fn get_files_and_folders(path: String) -> Pin<Box<dyn Future<Output = Vec<AppFile>> + Send>> {
        Box::pin(async move {
            let mut files: Vec<AppFile> = Vec::new();
            if let Ok(mut entries) = tokio::fs::read_dir(path).await {
                while let Some(entry) = entries.next_entry().await.unwrap() {
                    let metadata = entry.metadata().await.unwrap();
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let is_hidden = is_hidden_entry(&file_name, &metadata);
                    if entry.file_type().await.unwrap().is_dir() {
                        let children = Self::get_files_and_folders(entry.path().to_str().unwrap().to_string()).await;
                        files.push(AppFile::new_with_children(entry.path().to_str().unwrap().to_string(), metadata.len(), children, is_hidden));
                    } else {
                        files.push(AppFile::new(entry.path().to_str().unwrap().to_string(), metadata.len(), false, is_hidden));
                    }
                }
            }
            files
        })
    }

    pub async fn create_new_folder(path: String, name: String) -> Result<AppFile, std::io::Error> {
        let full_path = format!("{}/{}", path, name);
        let created = tokio::fs::create_dir_all(full_path).await;
        if created.is_ok() {
            Ok(AppFile::new(path.clone(), 0, true, false))
        } else {
            Err(created.err().unwrap())
        }
    }

    pub async fn create_new_file(path: String, file_name: String) -> Result<AppFile, std::io::Error> {
        let complete_path = path.clone() + "/" + &*file_name;
        let created = tokio::fs::File::create(complete_path.clone()).await;
        if created.is_ok() {
            Ok(AppFile::new(complete_path.clone(), 0, false, false))
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

    pub async fn move_files(sources: Vec<String>, destination: String) -> Result<(), std::io::Error> {
        for source in sources {
            let last = source.split("/").last();
            if let Some(name) = last {
                let final_path = destination.clone() + "/" + name;
                tokio::fs::rename(source, final_path).await?;
            }
        }
        Ok(())
    }
}

impl AppFile {
    pub fn new(path: String, size: u64, is_dir: bool, is_hidden: bool) -> Self {
        Self {
            path,
            size,
            is_dir,
            is_hidden,
            children: None,
        }
    }

    pub fn new_with_children(path: String, size: u64, children: Vec<AppFile>, is_hidden: bool) -> Self {
        Self {
            path,
            size,
            is_dir: true,
            is_hidden,
            children: Some(children),
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



