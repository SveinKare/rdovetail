use core::panic;
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::collections::HashMap;
use std::fs;
use std::env;
use crate::common::error::EntryConflict;

#[derive(Debug)]
pub struct FileData {
    hash: [u8; 32],
    path_from_root: Box<Path>,
}

impl FileData {
    pub fn new() -> Self {
        FileData {
            hash: [0; 32],
            path_from_root: Path::new(".").into(),
        }
    }

    pub fn set_hash(&mut self, hash: [u8; 32]) {
        self.hash = hash;
    }

    pub fn get_path_from_root(&self) -> PathBuf {
        self.path_from_root.to_path_buf()
    }

    pub fn set_path_from_root(&mut self, path: PathBuf) {
        self.path_from_root = path.into_boxed_path();
    }

    fn serialize(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        let mut path = String::new();
        for dir in self.path_from_root.iter() {
            let dir_str = match dir.to_str() {
                Some(str) => str,
                None => {
                    eprintln!("Filepath is not valid UTF-8.");
                    panic!();
                }
            };
            path.push_str(dir_str);
            path.push_str("/");
        }
        path.pop();
        let path = path.as_bytes();

        let data_length: u32 = (path.len() + 32).try_into().unwrap_or_else(|_| {
            panic!("Conversion from usize to u32 failed.");
        });
        let data_length = data_length.to_be_bytes();

        for byte in data_length {
            bytes.push(byte);
        }

        for byte in path {
            bytes.push(*byte);
        }

        for byte in self.hash {
            bytes.push(byte);
        }

        bytes
    }

    fn deserialize(bytes: &[u8]) -> Self {
        let mut hash: [u8; 32] = [0; 32];
        hash.copy_from_slice(&bytes[bytes.len()-32..bytes.len()]);

        let path = &bytes[0..bytes.len()-32];

        let path = match std::str::from_utf8(path) {
            Ok(path) => path,
            Err(err) => {
                panic!("Conversion from bytes to UTF-8 failed: {}", err);
            }
        };
        let mut path_from_root = PathBuf::new();
        for dir in path.split("/") {
            path_from_root.push(dir);
        }
        let path_from_root: Box<Path> = path_from_root.into_boxed_path();

        let file_data = FileData {
            hash,
            path_from_root,
        };
        file_data
    }

    fn display_hash(&self) -> String {
        let bytes = &self.hash;
        let mut hex_string = String::new();

        let hex_values = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'];

        let left_mask: u8 = 240;
        let right_mask: u8 = 15;
        for byte in bytes {
            let mut left_index = byte&left_mask;
            left_index >>= 4;
            hex_string.push(hex_values[usize::from(left_index)]);
            hex_string.push(hex_values[usize::from(byte&right_mask)]);
        }
        hex_string
    }
}

impl PartialEq for FileData {
    fn eq(&self, other: &Self) -> bool {
        self.hash.eq(&other.hash) && self.path_from_root.eq(&other.path_from_root)
    }
}

impl Clone for FileData {
    fn clone(&self) -> Self {
        let mut copy = FileData::new();
        copy.hash.copy_from_slice(&self.hash);
        copy.path_from_root = self.path_from_root.clone();
        copy
    }

}

pub struct Index {
    file_data: HashMap<[u8; 32], FileData>, 
    path_to_dir: Box<Path>,
}

impl Index {
    pub fn new() -> Self {
        let path_to_dir = match env::current_dir() {
            Ok(path) => path.into_boxed_path(),
            Err(_) => Path::new("./").to_path_buf().into_boxed_path(),
        };
        Index {
            file_data: HashMap::new(),
            path_to_dir,
        }
    }

    pub fn get_path_to_dir(&self) -> &Path {
        &self.path_to_dir
    }

    fn refresh() -> Self {
        // TODO: Scans entire directory and adds filedata file by file

        Self::new()
    }

    pub fn from_file(path: &Path) -> Self {
        let index = match Self::deserialize(&path) {
            Ok(index) => index,
            Err(err) => {
                eprintln!("Could not parse index file: {}", err);
                Index::refresh()
            }
        };

        index
    }

    // TODO: Buffer changes before writing to file to improve performance
    pub fn add_file_data(&mut self, relative_path_hash: [u8; 32], file_data: FileData) -> Result<(), EntryConflict> {
        let res = self.file_data.insert(relative_path_hash, file_data);
        match res {
            Some(_) => Err(EntryConflict{}),
            None => Ok(())
        }
    }

    pub fn remove_file_data(&mut self, relative_path_hash: [u8; 32]) -> Option<FileData> {
        self.file_data.remove(&relative_path_hash)
    }

    pub fn edit_file_data() {

    }

    pub fn write_to_file(&self) -> Result<(), io::Error>{
        let content = Self::serialize(&self);
        let mut file = fs::File::create(&self.path_to_dir.join(".rdovetail").join("index"))?;

        file.write_all(&content)?;

        Ok(())
    }

    fn serialize(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();

        for (hash, file_data) in self.file_data.iter() {
            for byte in hash {
                bytes.push(*byte);
            }

            for byte in file_data.serialize() {
                bytes.push(byte);
            }
        }

        bytes
    }

    fn deserialize(path: &Path) -> io::Result<Self> {
        let mut index = Index::new();

        let bytes = fs::read(path)?;

        let mut slice_start = 0;

        while slice_start < bytes.len() {
            // Read key hash
            let mut hash: [u8; 32] = [0; 32];
            hash.copy_from_slice(&bytes[slice_start..slice_start+32]);
            slice_start += 32;

            // Read file data and deserialize FileData struct
            let mut bytes_to_read: [u8; 4] = [0; 4];
            bytes_to_read.copy_from_slice(&bytes[slice_start..slice_start+4]);
            let bytes_to_read: usize = u32::from_be_bytes(bytes_to_read)
                .try_into()
                .map_err(|_| io::Error::new(
                        io::ErrorKind::InvalidData, 
                        "Failed to convert u32 to usize."
                        ))?;
            slice_start += 4;
            let file_data = FileData::deserialize(&bytes[slice_start..slice_start+bytes_to_read]);

            index.file_data.insert(hash, file_data);
            slice_start += bytes_to_read;
        }
        Ok(index)
    } 
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::path::Path;
    use crate::common::util::create_file_data;

    #[test]
    fn copy_and_clone_works_for_file_data() {
        let mut original = FileData::new();
        original.hash = {
            let mut array = [0; 32];
            for i in 0..32 {
                array[i] = 1;
            }
            array
        };
        let clone = original.clone();
        assert_eq!(original, clone);
    }

    #[test]
    fn index_is_serialized() -> Result<(), io::Error> {
        let mut index = Index::new();
        let _ = fs::create_dir("./test");
        let _ = fs::create_dir("./test/.rdovetail");
        let mut file_handle = fs::File::create("./test/test_data.txt")?;
        let _ = file_handle.write(b"asdf");
        let mut hasher = Sha256::new();
        hasher.update(b".testtest_data.txt");
        let relative_path_hash: [u8; 32] = hasher.finalize().into();
        let file_data = match create_file_data(
            PathBuf::from("."),
            PathBuf::from("./test/test_data.txt")
            ) {
            Some(data) => data,
            None => return Err(io::Error::new(io::ErrorKind::Other, "Failed to create FileData")),
        };
        let file_data_clone = file_data.clone();
        let res = index.add_file_data(relative_path_hash, file_data);
        assert!(res.is_none());
        index.path_to_dir = Path::new("./test").to_owned().into_boxed_path();
        let _ = index.write_to_file()?;
        let read_index = Index::from_file(Path::new("./test/.rdovetail/index"));
        let entry = read_index.file_data.get(&relative_path_hash);
        assert!(entry.is_some(), "Entry in read index does not exist.");
        assert_eq!(entry.unwrap(), &file_data_clone, "Read filedata does not match original.");
        Ok(())
    }
}
