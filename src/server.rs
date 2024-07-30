use std::net::TcpListener;
use std::io::{BufRead, BufReader};
use crate::Config;

pub fn init(config: &Config) {
    let listener = TcpListener::bind(config.address).unwrap();
    println!("Hello from server");
    for stream in listener.incoming() {
        let socket = stream.unwrap();
        let mut reader = BufReader::new(socket);
        let mut message = String::new();

        let _ = reader.read_line(&mut message);
        
        println!("Connection made.");
        println!("Message: {}", &message);
    }
}
