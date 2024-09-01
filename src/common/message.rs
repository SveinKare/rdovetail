use std::path::PathBuf;

#[derive(Debug)]
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
    ExternalChange {
        
    }
}

