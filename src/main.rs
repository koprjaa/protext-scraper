// Project: protext-scraper
// File: main.rs
// Description: Interactive executable entry point.
// Author: Jan Alexandr Kopřiva
// License: MIT

use protext_scraper::{app, network::Network};
fn main() {
    let _ = ctrlc::set_handler(|| {
        println!("\n\nBye!");
        std::process::exit(0);
    });
    match std::env::current_dir()
        .and_then(|root| app::run(&Network::default(), &mut std::io::stdin().lock(), &root))
    {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => println!("\n\nBye!"),
        Err(e) => println!("Error during choice: {e}"),
    }
}
