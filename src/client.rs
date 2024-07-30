use notify::{Result};
use crate::common::{version_control, message::Message};
use std::sync::mpsc::{channel, Sender, Receiver};

pub fn init() -> Result<()> {
    let (tx_to_vcs, rx_vcs): (Sender<Message>, Receiver<Message>) = channel();
    let (tx_vcs, rx_from_vcs): (Sender<Message>, Receiver<Message>) = channel();

    let _ = version_control::start(tx_vcs, rx_vcs, tx_to_vcs.clone());

    Ok(())
}
