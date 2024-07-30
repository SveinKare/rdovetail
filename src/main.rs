use std::process;
use std::net::{AddrParseError, SocketAddr};
use std::str::FromStr;
use clap::Parser;

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
    let args = args.unwrap_or_else(|err| {
        println!("{}", err);
        process::exit(1);
    });

    let config = Config::build(args).unwrap_or_else(|_err| {
        println!("IP-address could not be parsed.");
        process::exit(1);
    });
    println!("Address: {}\nServer mode: {}", config.address, config.server_mode);

    if config.server_mode {
        server::init(&config);
    } else {
        client::init();
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The IP-address of the server being linked to
    #[arg(short, long)]
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
    fn build(args: Args) -> Result<Config, AddrParseError> {
        let address = SocketAddr::from_str(&args.ip)?;
        Ok(Config {
            address,
            server_mode: args.server_mode,
        })
    }
}
