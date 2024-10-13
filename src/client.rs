use notify::{Result};
use crate::common::{version_control, message::Message};
use crate::Config;
use core::panic;
use std::io::Write;
use std::net::TcpStream;

pub fn init(config: &Config) -> Result<()> {
    let res = version_control::start();
    let mut stream = TcpStream::connect(config.address)?;
    
    let (tx_to_vcs, rx_from_vcs) = match res {
        Ok((tx, rx)) => (tx, rx),
        Err(err) => panic!("An error occurred: {:?}", err),
    };
    println!("Ready");

    let message = rx_from_vcs.recv().unwrap();
    let encoded = bincode::serialize(&message).unwrap();
    let message_length = encoded.len() as u64;
    stream.write_all(&message_length.to_be_bytes())?;
    stream.write_all(&encoded)?;
    stream.flush()?;

    println!("Message sent");
    
    Ok(())
}
