use notify::{Result};
use crate::common::{version_control, message::Message};
use core::panic;
use std::sync::mpsc::{channel, Sender, Receiver};

pub fn init() -> Result<()> {
    let (tx_to_vcs, rx_vcs): (Sender<Message>, Receiver<Message>) = channel();
    let (tx_vcs, rx_from_vcs): (Sender<Message>, Receiver<Message>) = channel();

    let res = version_control::start(tx_vcs, rx_vcs, tx_to_vcs.clone());
    match res {
        Ok(_) => Ok(()),
        Err(err) => panic!("An error occurred in version_control::start(): {:?}", err),
    }
}
