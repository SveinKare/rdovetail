use core::panic;
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::collections::HashMap;
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::hash::Hash;
use sha2::{Digest, Sha256};
use crate::common::error::EntryConflict;
use super::util::{as_nanos_since_epoch};

#[derive(Debug)]
pub struct FileData {
    hash: [u8; 32],
    path_from_root: Box<Path>,
    timestamp: SystemTime,
}

impl FileData {
    pub fn new() -> Self {
        FileData {
            hash: [0; 32],
            path_from_root: Path::new(".").into(),
            timestamp: UNIX_EPOCH,
        }
    }

    pub fn get_hash(&self) -> &[u8; 32] {
        &self.hash
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

    pub fn get_timestamp(&self) -> &SystemTime {
        &self.timestamp
    }

    pub fn set_timestamp(&mut self, timestamp: SystemTime) {
        self.timestamp = timestamp
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

        let data_length: u32 = (path.len() + 40).try_into().unwrap_or_else(|_| {
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

        let nanos_since_epoch = as_nanos_since_epoch(&self.timestamp);
        for byte in nanos_since_epoch.to_be_bytes() {
            bytes.push(byte);
        }

        bytes
    }

    fn deserialize(bytes: &[u8]) -> Self {
        // Last 48 bytes are hash and timestamp, everything up until that point is the path
        let path = &bytes[0..bytes.len()-40];
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

        // SHA256 hash of the files content and filename
        let mut hash: [u8; 32] = [0; 32];
        hash.copy_from_slice(&bytes[bytes.len()-40..bytes.len()-8]);

        // Last 8 bytes is the ms since epoch timestamp
        let ms_since_epoch: [u8; 8] = bytes[bytes.len()-8..bytes.len()].try_into().unwrap();
        let ms_since_epoch = u64::from_be_bytes(ms_since_epoch);
        let ms_since_epoch = Duration::from_nanos(ms_since_epoch);
        let timestamp = UNIX_EPOCH.checked_add(ms_since_epoch).unwrap();

        let file_data = FileData {
            hash,
            path_from_root,
            timestamp,
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
        self.hash.eq(&other.hash) 
            && self.path_from_root.eq(&other.path_from_root)
            && self.timestamp.eq(&other.timestamp)
    }
}

impl Clone for FileData {
    fn clone(&self) -> Self {
        let mut copy = FileData::new();
        copy.hash.copy_from_slice(&self.hash);
        copy.path_from_root = self.path_from_root.clone();
        copy.timestamp = self.timestamp.clone();
        copy
    }
}

#[derive(Eq, PartialEq, Debug)]
struct SHA256Hash {
    value: [u8; 32],
}

impl Hash for SHA256Hash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.value);
    }
}

/// An overview of the currently tracked files in the directory. 
#[derive(Debug)]
pub struct Index {
    file_data: HashMap<SHA256Hash, FileData>, 
    path_to_dir: PathBuf,
}

impl Index {
    pub fn new(path_to_dir: PathBuf) -> Self {
        Index {
            file_data: HashMap::new(),
            path_to_dir,
        }
    }

    pub fn get_path_to_dir(&self) -> &Path {
        &self.path_to_dir
    }

    pub fn get_current_state(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        let mut data: Vec<u8> = Vec::new();
        for (_key, value) in self.file_data.iter() {
            for byte in value.hash {
                data.push(byte);
            }
        }
        hasher.update(data);
        hasher.finalize().into()
    }

    pub fn from_file(path: &Path) -> io::Result<Self> {
        Self::deserialize(&path)
    }

    pub fn get_file_data(&self, key: &[u8; 32]) -> Option<&FileData> {
        let key = SHA256Hash {
            value: *key,
        };
        self.file_data.get(&key)
    }

    pub fn add_file_data(&mut self, relative_path_hash: [u8; 32], file_data: FileData) -> Result<(), EntryConflict> {
        let key = SHA256Hash {
            value: relative_path_hash,
        };
        let res = self.file_data.insert(key, file_data);
        match res {
            Some(_) => Err(EntryConflict{}),
            None => Ok(())
        }
    }

    pub fn remove_file_data(&mut self, relative_path_hash: [u8; 32]) -> Option<FileData> {
        let key = SHA256Hash {
            value: relative_path_hash,
        };
        let return_value = self.file_data.remove(&key);
        return_value
    }

    pub fn edit_file_data(&mut self) {
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
            for byte in hash.value {
                bytes.push(byte);
            }

            for byte in file_data.serialize() {
                bytes.push(byte);
            }
        }

        bytes
    }

    fn deserialize(path: &Path) -> io::Result<Self> {
        let mut path_to_dir = path.to_path_buf();
        path_to_dir.pop();
        path_to_dir.pop();
        let mut index = Index::new(path_to_dir);

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

            let _ = index.add_file_data(hash, file_data);
            slice_start += bytes_to_read;
        }
        Ok(index)
    } 
}

pub enum ChangeType {
    Create {
        file_hash: [u8; 32],
    },
    Delete,
    Modify {
        file_hash: [u8; 32],
    },
    Rename {
        new_name: String,
    },
}

pub struct Change {
    pub change_type: ChangeType,
    pub new_state: [u8; 32],
    pub timestamp: u64,
    pub file_path: PathBuf,
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
        original.set_timestamp(UNIX_EPOCH.checked_add(Duration::from_millis(5000)).unwrap());
        let clone = original.clone();
        assert_eq!(original, clone);
    }

    #[test]
    fn index_is_serialized() -> Result<(), io::Error> {
        let mut index = Index::new(PathBuf::from("./test"));
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
        assert!(res.is_ok());
        let _ = index.write_to_file()?;
        let read_index = Index::from_file(Path::new("./test/.rdovetail/index"))?;
        let relative_path_hash = SHA256Hash {
            value: relative_path_hash,
        };
        let entry = read_index.file_data.get(&relative_path_hash);
        assert!(entry.is_some(), "Entry in read index does not exist.");
        assert_eq!(entry.unwrap(), &file_data_clone, "Read filedata does not match original.");
        Ok(())
    }
}
