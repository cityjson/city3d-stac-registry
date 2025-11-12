use cityjson_stac::cli;
use std::process;

fn main() {
    env_logger::init();

    if let Err(e) = cli::run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
