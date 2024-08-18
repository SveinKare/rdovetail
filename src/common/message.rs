use std::path::PathBuf;

pub enum Message {
    FileCreated {
        path: PathBuf,
    },
    FileRemoved {
        path: PathBuf,
    },
    FileRequest {
        relative_path_hash: [u8; 32],
    },
}

