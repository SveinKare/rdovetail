use sha2::{Sha256, Digest};
use std::path::{Path, PathBuf, Iter};
use std::fs;

use super::data::{FileData};

pub fn hash_path(path: &Path) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for dir_name in path.iter() {
        Digest::update(&mut hasher, dir_name.as_encoded_bytes());
    }

    let relative_path_hash: [u8; 32] = hasher.finalize().into();
    relative_path_hash
}

pub fn hash_file(path: &Path) -> Option<[u8; 32]> {
    let mut content = match fs::read(&path) {
        Ok(content) => content,
        Err(_) => return None,
    };

    let file_name = match path.file_name() {
        Some(name) => name,
        None => return None,
    };

    for byte in file_name.as_encoded_bytes() {
        content.push(*byte);
    }

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();

    let mut returned: [u8; 32] = [0; 32];
    let mut index = 0;
    for byte in result.iter() {
        returned[index] = *byte;
        index += 1;
    }
    Some(returned)
}

pub fn create_file_data(path_to_dir: PathBuf, path: PathBuf) -> Option<FileData> {
    let hash = match hash_file(&path) {
        Some(hash) => hash,
        None => {
            println!("Hash failed!");
            return None;
        },
    };
    let mut file_data = FileData::new();
    file_data.set_hash(hash);
    file_data.set_path_from_root(find_relative_path(
            path_to_dir.iter(), 
            path.iter()
            ));

    Some(file_data)
}


pub fn find_relative_path(path_to_dir: Iter<>, mut path: Iter<>) -> PathBuf {
    for _ in path_to_dir {
        path.next();
    }
    let mut relative_path = PathBuf::new();
    relative_path.push(".");

    for path_str in path {
        relative_path.push(path_str);
    }

    relative_path
}
