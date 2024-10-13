use std::net::TcpListener;
use std::io::{BufRead, BufReader, Read};
use crate::common::message::Message;
use crate::Config;

pub fn init(config: &Config) {
    let listener = TcpListener::bind(config.address).unwrap();
    for stream in listener.incoming() {
        let socket = stream.unwrap();
        let mut reader = BufReader::new(socket);
        let mut buffer: Vec<u8> = vec![];

        let read = reader.read(&mut buffer).unwrap();
        buffer.truncate(read);
        let message: Message = bincode::deserialize(&buffer).unwrap();

        
        println!("Connection made.");
        println!("Message: {:?}", &message);
    }
}
