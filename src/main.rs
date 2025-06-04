use colored::*;
use std::env;
use std::process;
use flerp::Config;
fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::new(&args).unwrap_or_else(|err| {
        eprint!(
            "{}",
            "Who scrambled the arguments? -> ".bright_yellow().bold()
        );

        eprintln!("{}", err.bright_red().bold());
        process::exit(1);
    });

    println!(
        "{} {}",
        "Searching for:".bright_blue().bold(),
        config.query.bright_white().underline()
    );

    println!(
        "{} {}",
        "In file:".bright_green().bold(),
        config.file_name.bright_white().underline()
    );

    if let Err(e) = flerp::run(config) {
        eprintln!(
            "{}",
            format!("App error: {}", e.to_string()).bright_red().bold()
        );
        process::exit(1);
    }
}


