use std::path::PathBuf;

pub enum Message {
    FileCreated {
        path: PathBuf,
    },
    FileRemoved {
        path: PathBuf,
    },
}

