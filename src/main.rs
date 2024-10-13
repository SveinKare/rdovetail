use std::process;
use std::net::{AddrParseError, IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use clap::Parser;
use common::error::IllegalState;

pub mod server;
pub mod client;
pub mod common {
    pub mod version_control;
    pub mod message;
    pub mod util;
    pub mod data;
    pub mod error;
}

fn main() {
    let args = Args::try_parse();
    let args = args.unwrap_or_else(|help_message| {
        println!("{}", help_message); // For the info flags like help and version
        process::exit(0);
    });

    let config = match Config::build(args) {
        Ok(config) => config,
        Err(err) => {
            println!("{}", err);
            process::exit(1);
        }
    };

    println!("Address: {}\nServer mode: {}", config.address, config.server_mode);

    if config.server_mode {
        server::init(&config);
    } else {
        let res = client::init(&config);
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The IP-address of the server being linked to
    #[arg(short, long, default_value_t = String::new())]
    ip: String,

    /// Indicates that the current machine should act as a server, and receive incoming connections
    /// from other machines.
    #[arg(short, action)]
    server_mode: bool,
}

pub struct Config {
    address: SocketAddr, 
    server_mode: bool,
}

impl Config {
    fn build(args: Args) -> Result<Config, IllegalState> {
        let address = match SocketAddr::from_str(&args.ip) {
            Ok(addr) => Some(addr),
            Err(_) => None,
        };

        if !args.server_mode && address.is_none() {
            return Err(IllegalState::new("Invalid ip for client mode".to_string()))
        }

        Ok(Config {
            address: address.unwrap_or(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 50010))),
            server_mode: args.server_mode,
        })

    }
}
