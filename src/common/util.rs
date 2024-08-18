use sha2::{Sha256, Digest};
use core::panic;
use std::path::{Path, PathBuf, Iter};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::{fs, io, thread};
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs::File;
use std::thread::JoinHandle;
use memmap2::{Mmap, MmapOptions};

use super::data::{FileData, Index};

pub fn hash_path(path: &PathBuf) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for dir_name in path.iter() {
        Digest::update(&mut hasher, dir_name.as_encoded_bytes());
    }

    let relative_path_hash: [u8; 32] = hasher.finalize().into();
    relative_path_hash
}

pub fn hash_file(path: &Path) -> Option<[u8; 32]> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return None,
    };
    let buffer_size: usize = 512;
    
    let mmap = unsafe {
        let opt = MmapOptions::new().len(buffer_size).map(&file);
        match opt {
            Ok(m) => m,
            Err(_) => match Mmap::map(&file) {
                Ok(map) => map,
                Err(err) => panic!("File could not be read: {:?}", err),
            },
        }
    };

    let mut hasher = Sha256::new();
    hasher.update(&mmap);
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
    match path.metadata() {
        Ok(metadata) => {
            file_data.set_timestamp(metadata.modified().unwrap());
        },
        Err(err) => panic!("Failed to read timestamp from metadata: {:?}", err),
    }

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

fn find_all_files(path_to_dir: &PathBuf, filepaths: &mut Vec<PathBuf>) {
    let read_dir = fs::read_dir(path_to_dir);
    let dir = match read_dir {
        Ok(dir) => dir,
        Err(err) => panic!("Error matching read_dir: {:?}", err),
    };
    for d in dir {
        let entry = match d {
            Ok(entry) => entry,
            Err(err) => panic!("Error matching entries: {:?}", err),
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(err) => panic!("Error getting file type: {:?}", err),
        };

        if file_type.is_dir() { // Current entry is a directory, do a recursive call
            find_all_files(&path, filepaths);
        } else { // Current entry is a file, add path for bulk processing
            filepaths.push(path);
        }
    }
}

pub fn index_from_dir(path_to_dir: &PathBuf, index: Arc<Mutex<Index>>) -> io::Result<()> {
    let mut filepaths = Vec::new();
    let start = SystemTime::now();
    find_all_files(path_to_dir,&mut filepaths);
    let checkpoint = SystemTime::now();
    let (tx, rx): (Sender<([u8; 32], FileData)>, Receiver<([u8; 32], FileData)>) = channel();

    let thread_amount = 4;
    let mut thread_handles: Vec<JoinHandle<()>> = Vec::new();
    let slice_size = filepaths.len()/thread_amount;
    for i in 0..thread_amount {
        let filepaths_slice = match i.cmp(&(thread_amount-1)) {
            std::cmp::Ordering::Equal => {
                Vec::from_iter(filepaths[i*slice_size..].iter().cloned())
            },
            _other => {
                Vec::from_iter(filepaths[i*slice_size..(i+1)*slice_size].iter().cloned())
            },
        };
        let path_to_dir_clone = path_to_dir.clone();
        let tx_clone = tx.clone();

        let handle = thread::spawn(move || {
            for path in filepaths_slice {
                let key = hash_path(&find_relative_path(path_to_dir_clone.iter(), path.iter()));
                let data = create_file_data(path_to_dir_clone.to_path_buf(), path.to_path_buf()).unwrap();
                if let Err(err) = tx_clone.send((key, data)) {
                    panic!("An error occured while processing filedata: {:?}", err);
                };
            }
        });
        thread_handles.push(handle);
    }
    drop(tx);
    let thread_index = Arc::clone(&index);
    thread_handles.push(thread::spawn(move || {
        for (key, data) in rx {
            match thread_index.lock().unwrap().add_file_data(key, data) {
                Ok(_) => continue,
                Err(err) => panic!("Duplicate key: {:?}", err),
            };
        }
    }));
    for thread in thread_handles {
        let _ = thread.join();
    }

    let finished_processing = SystemTime::now();

    println!("Time to read files: {:?} ms", checkpoint.duration_since(start).unwrap().as_millis());
    println!("Time to process files: {:?} ms", finished_processing.duration_since(checkpoint).unwrap().as_millis());
    Ok(())
}

pub fn as_nanos_since_epoch(system_time: &SystemTime) -> u64 {
    let dur_since_epoch = system_time.duration_since(UNIX_EPOCH);
    match dur_since_epoch {
        Ok(dur) => {
            dur.as_nanos().try_into().expect("Failed to convert timestamp to u64.")
        },
        Err(err) => panic!("Failed to get duration since epoch: {:?}", err),
    }
}
