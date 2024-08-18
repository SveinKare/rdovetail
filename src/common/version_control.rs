use notify::{EventHandler, EventKind, RecursiveMode, Watcher};
use std::env;
use std::path::Path;
use std::error::Error;
use std::fs::create_dir;
use std::sync::mpsc::{Sender, Receiver, SendError};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::common::{
    message::Message, 
    data::{Index, FileData, Change, ChangeType}, 
    util::{hash_path, create_file_data, find_relative_path, as_nanos_since_epoch}
};

use super::error::IllegalState;
use super::util::index_from_dir;

fn init_dovetail(dir: &Path) -> Result<Index, Box<dyn Error>> {
    let dovetail_dir = &dir.join(".rdovetail");
    let dovetail_initialized = dovetail_dir.try_exists().unwrap_or_else(|_| {
        false
    });
    let index = {
        if !dovetail_initialized {
            create_dir(dovetail_dir)?;  
            let temp = Index::new(dir.to_path_buf());
            let arc_temp = Arc::new(Mutex::new(temp));
            index_from_dir(&dir.to_path_buf(), Arc::clone(&arc_temp))?;
            let mtx = Arc::try_unwrap(arc_temp).unwrap();
            let index = mtx.into_inner().unwrap();
            index.write_to_file()?;
            index
        } else {
            Index::from_file(&dovetail_dir.join("index"))?
        }
    };
    Ok(index)
}

pub fn start(
    tx_alpha: Sender<Message>, 
    rx_beta: Receiver<Message>,
    tx_beta_clone: Sender<Message>,
    ) -> Result<(), Box<dyn Error>> { 
    let path = env::current_dir()?;

    let index = init_dovetail(&path)?;
    // Check entire directory, and ensure that index is up to date

    let mut watcher = notify::recommended_watcher(
        ChangeNotifier {
            tx: tx_beta_clone,
    })?;

    let mut vcs = VersionControl {
        index,
        rx_updates: rx_beta,
        tx_to_client: tx_alpha,
        changes: Vec::new(),
    };

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(&path, RecursiveMode::Recursive)?;

    println!("Started listening.");
    loop {
        vcs.listen();
    }
    Ok(())
}

// VCS should have some sort of data structure that can aid in determining if a file has been
// deleted.

struct VersionControl {
    index: Index,
    rx_updates: Receiver<Message>,
    tx_to_client: Sender<Message>,
    changes: Vec<Change>,
}

impl VersionControl {
    fn send_update(&self, message: Message) -> Result<(), SendError<Message>> {
        println!("Message sent to client.");
        self.tx_to_client.send(message)
    }

    fn listen(&mut self) {
        match self.rx_updates.recv() {
            Ok(message) => {
                match message {
                    Message::FileCreated { path } => {
                        println!("Created: {:?}", path);
                        
                        if let Ok(key) = self.add_file_data(&path) {
                            let file_hash = self.index.get_file_data(&key).unwrap().get_hash();
                            self.changes.push(Change {
                                change_type: ChangeType::Create {
                                    file_hash: *file_hash
                                },
                                new_state: self.index.get_current_state(),
                                timestamp: as_nanos_since_epoch(&SystemTime::now()),
                                file_path: path.clone(),
                            });
                            if let Err(err) = self.send_update(Message::FileCreated { path }) {
                                println!("Error: {:?}", err)
                                //check_health
                            }
                        } else {
                            return
                        }
                    },
                    Message::FileRemoved { path } => {
                        println!("Removed: {:?}", path);
                        if let Some(_) = self.remove_file_data(&path) {
                            self.changes.push(Change {
                                change_type: ChangeType::Delete,
                                new_state: self.index.get_current_state(),
                                timestamp: as_nanos_since_epoch(&SystemTime::now()),
                                file_path: path.clone(),
                            });
                            if let Err(err) = self.send_update(Message::FileRemoved { path }) {
                                println!("Error: {:?}", err)
                                //check_health
                            }
                        }
                    },
                    Message::FileRequest { relative_path_hash } => {


                    },
                }
            },
            Err(err) => println!("Error: {:?}", err),
        }
    }

    fn add_file_data(&mut self, path: &Path) -> Result<[u8; 32], Box<dyn Error>> {
        // Hashes content with filename
        let file_data = match create_file_data(
            self.index.get_path_to_dir().to_path_buf(), 
            path.to_path_buf()
        ) {
            Some(file_data) => file_data,
            None => return Err(Box::new(IllegalState::new("filedata could not be created".to_string()))),
        };

        // Uses the relative path to the file as a key to avoid collisions for
        // files with the same name.
        let relative_path_hash = hash_path(&file_data.get_path_from_root());

        self.index.add_file_data(relative_path_hash, file_data)?;
        self.index.write_to_file()?;
        Ok(relative_path_hash)
    }

    fn remove_file_data(&mut self, path: &Path) -> Option<FileData> {
        let relative_path = find_relative_path(
            self.index.get_path_to_dir().iter(), 
            path.iter()
        );

        let relative_path_hash = hash_path(&relative_path);
        self.index.remove_file_data(relative_path_hash)
    }

    fn implement_change(&mut self, change: Change) {
        match change.change_type {
            ChangeType::Create { file_hash } => {
                // Request file from original source
                //

            },
            ChangeType::Delete => {
                // Find the actual file and delete it
                self.remove_file_data(&change.file_path);
            },
            ChangeType::Modify { file_hash } => {

            },
            ChangeType::Rename { new_name } => {

            }
        }
    }
}

struct ChangeNotifier {
    tx: Sender<Message>,
}

impl ChangeNotifier {
    fn notify_vcs(&self, message: Message) {
        let _ = self.tx.send(message);
    }
}

impl EventHandler for ChangeNotifier {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        match event {
            Ok(event) => {
                // Avoids recursion from changes to .rdovetail files
                for path_buf in &event.paths {
                    for dir in path_buf.iter() {
                        if dir == ".rdovetail" {
                            return;
                        }
                    }
                }
                let path = &event.paths[0];

                match event.kind {
                    EventKind::Remove(_) => {
                        let message = Message::FileRemoved{
                            path: path.clone(),
                        };
                        self.notify_vcs(message);
                    },
                    EventKind::Create(_) => {
                        let message = Message::FileCreated { 
                            path: path.clone(),
                        };
                        self.notify_vcs(message);
                    }, 
                    _ => println!("Event type: {:?}", &event.kind),
                };
            },
            Err(err) => {
                eprintln!("Error occured in version_control.rs: {}", err);
                // check_health()?
            },
        }
    }
}

