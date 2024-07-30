use notify::{EventHandler, EventKind, RecursiveMode, Watcher};
use std::io;
use std::path::Path;
use std::error::Error;
use std::fs::create_dir;
use std::sync::mpsc::{Sender, Receiver, SendError, RecvTimeoutError, channel};

use crate::common::{
    message::Message, 
    data::{Index, FileData}, 
    util::{hash_path, create_file_data, find_relative_path}
};

use super::error::IllegalState;


fn init(dovetail_dir: &Path) -> Result<Index, Box<dyn Error>> {
    let dovetail_initialized = dovetail_dir.try_exists().unwrap_or_else(|_| {
        false
    });
    let mut index: Option<Index> = None;
    if !dovetail_initialized {
        create_dir(dovetail_dir)?;  
        index = Some(Index::new());
    }
    let index: Index = match index {
        Some(value) => value,
        None => Index::from_file(&dovetail_dir.join("index")),
    };
    Ok(index)
}

pub fn start(
    tx_to_client: Sender<Message>, 
    rx_from_client: Receiver<Message>,
    tx_from_client_clone: Sender<Message>,
    ) -> Result<(), Box<dyn Error>> { 
    let path = Path::new(".");
    let dovetail_dir = &path.join(".rdovetail");

    let index = init(&dovetail_dir)?;
    // Check entire directory, and ensure that index is up to date

    // Swap out VersionControl with a separate handler. This handler will have a transmitter to
    // request changes to index. THe transmitter will be cloned, and used in the loop below. Both
    // the client file and handler can request changes to the index, with a first come first serve
    // basis. 
    let mut watcher = notify::recommended_watcher(
        ChangeNotifier {
            tx: tx_from_client_clone,
        })?;

    let mut vcs = VersionControl {
        index,
        rx_from_client,
        tx_to_client,
    };

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(&path, RecursiveMode::Recursive)?;

    loop {
        vcs.listen();
    }
    Ok(())
}

struct VersionControl {
    index: Index,
    rx_from_client: Receiver<Message>,
    tx_to_client: Sender<Message>,
}

impl VersionControl {
    fn send_update(&self, message: Message) -> Result<(), SendError<Message>> {
        println!("Message sent to client.");
        self.tx_to_client.send(message)
    }

    fn listen(&mut self) {
        match self.rx_from_client.recv() {
            Ok(message) => {
                match message {
                    Message::FileCreated { path } => {
                        println!("Created: {:?}", path);
                        if let Ok(_) = self.add_file_data(&path) {
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
                            if let Err(err) = self.send_update(Message::FileRemoved { path }) {
                                println!("Error: {:?}", err)
                                //check_health
                            }
                        }
                    },
                }
            },
            Err(err) => println!("Error: {:?}", err),
        }
    }

    fn add_file_data(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
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
        Ok(())
    }

    fn remove_file_data(&mut self, path: &Path) -> Option<FileData> {
        let relative_path = find_relative_path(
            self.index.get_path_to_dir().iter(), 
            path.iter()
        );

        let relative_path_hash = hash_path(&relative_path);
        self.index.remove_file_data(relative_path_hash)
    }
}

struct ChangeNotifier {
    tx: Sender<Message>,
}

impl ChangeNotifier {
    fn notify_vcs(&self, message: Message) {
        self.tx.send(message);
    }
}

// Check if change happened in .rdovetail, ignore if thats the case - COMPLETE
// Hash file - COMPLETE
// Check if file hash is different from existing one
// Add to index if not already there, update if it is
// (Optional) Compress file
// Sync with server
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

