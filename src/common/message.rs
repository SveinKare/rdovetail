use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
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

