use notify::{Result};
use crate::common::{version_control, message::Message};
use crate::Config;
use core::panic;
use std::io::Write;
use std::net::TcpStream;

pub fn init(config: &Config) -> Result<()> {
    let res = version_control::start();
    
    let (tx_to_vcs, rx_from_vcs) = match res {
        Ok((tx, rx)) => (tx, rx),
        Err(err) => panic!("An error occurred: {:?}", err),
    };

    let mut stream = TcpStream::connect(config.address)?;
    let test = rx_from_vcs.recv().unwrap();
    
    println!("{:?}", test);
    Ok(())
}
