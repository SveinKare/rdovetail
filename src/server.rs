use std::net::TcpListener;
use std::io::{BufRead, BufReader, Read};
use crate::common::message::Message;
use crate::Config;

pub fn init(config: &Config) {
    let listener = TcpListener::bind(config.address).unwrap();
    for stream in listener.incoming() {
        println!("Connection made");
        let socket = stream.unwrap();
        let mut reader = BufReader::new(socket);

        let mut message_length = [0u8; 8];
        reader.read_exact(&mut message_length).unwrap();
        let mut buffer: Vec<u8> = vec![0u8; u64::from_be_bytes(message_length) as usize];

        let read = reader.read_exact(&mut buffer).unwrap();
        let message: Message = bincode::deserialize(&buffer).unwrap();

        println!("Message: {:?}", &message);
    }
}
