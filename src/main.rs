use colored::*;
use std::env;
use std::error::Error;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::new(&args).unwrap_or_else(|err| {
        print!(
            "{}",
            "Who scrambled the arguments? -> ".bright_yellow().bold()
        );

        println!("{}", err.bright_red().bold());
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

    if let Err(e) = run(config) {
        println!(
            "{}",
            format!("App error: {}", e.to_string()).bright_red().bold()
        );
        process::exit(1);
    }
}

fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(config.file_name)?;

    // Display file contents with a bright cyan label
    println!("{} \n{}", "With text:".bright_cyan().bold(), contents);

    Ok(())
}

struct Config {
    query: String,
    file_name: String,
}

impl Config {
    fn new(args: &[String]) -> Result<Config, &str> {
        if args.len() < 3 {
            return Err("Looks like you passed some wonky doohickeys");
        }
        let query = args[1].clone();
        let file_name = args[2].clone();

        Ok(Config { query, file_name })
    }
}
