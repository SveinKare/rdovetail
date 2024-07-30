use std::error::Error;
use core::fmt::Display;

#[derive(Debug)]
pub struct EntryConflict {}


impl Display for EntryConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "an entry of that description already exists")
    }

}

impl Error for EntryConflict {}

#[derive(Debug)]
pub struct IllegalState {
    message: String,
}
impl IllegalState {
    pub fn new(message: String) -> Self {
        IllegalState {
            message
        }
    }
}

impl  Display for IllegalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.message)
    }
}

impl Error for IllegalState {}
