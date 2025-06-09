use colored::*;
use std::env;
use std::error::Error;
use std::fs;

// Declare modules that are part of this library crate
pub mod app_structs; // Assuming app_structs.rs is part of the library
pub mod text_analysis;

// Use items from declared modules
use crate::text_analysis::{analyze_structure, extract_keywords, search, search_case_insensitive};
// Import StructuralAnalysisResults directly from its own module

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let contents = if config.file_name.ends_with(".pdf") {
        println!("{}", "Reading pdf file...".bright_blue().bold());
        pdf_extract::extract_text(&config.file_name)?
    } else {
        fs::read_to_string(&config.file_name)? // Corrected to pass a reference
    };
    let structural_analysis_results = analyze_structure(&contents);
    println!("{}", "Structural Analysis:".cyan().bold());
    println!(
        "  Lines: {}",
        structural_analysis_results.lines.to_string().yellow()
    );
    println!(
        "  Words: {}",
        structural_analysis_results.words.to_string().yellow()
    );
    println!(
        "  Characters: {}",
        structural_analysis_results.characters.to_string().yellow()
    );
    println!(
        "  Stanzas: {}",
        structural_analysis_results.stanzas.to_string().yellow()
    );

    let keywords = extract_keywords(&contents, 5);
    println!("\n{}", "Keywords (Top 5):".cyan().bold());
    for (keyword, count) in keywords {
        println!("  {}: {}", keyword.green().bold(), count.to_string().blue());
    }

    println!("\n{}", "Search Results:".cyan().bold());
    let search_results_owned: Vec<String> = if config.case_sensitive {
        search(&config.query, &contents)
    } else {
        search_case_insensitive(&config.query, &contents)
    };

    if search_results_owned.is_empty() {
        println!("{}", "  No matches found.".italic());
    } else {
        let search_results_refs: Vec<&str> =
            search_results_owned.iter().map(|s| s.as_str()).collect();

        for line_content_str in search_results_refs {
            let query_to_highlight = &config.query;
            if config.case_sensitive {
                let mut last_end = 0;
                for (start, part) in line_content_str.match_indices(query_to_highlight) {
                    print!("{}", &line_content_str[last_end..start]);
                    print!("{}", part.magenta().bold());
                    last_end = start + part.len();
                }
                println!("{}", &line_content_str[last_end..]);
            } else {
                let lower_query = query_to_highlight.to_lowercase();
                let mut last_end = 0;
                let mut current_search_pos = 0;
                let lower_line_content = line_content_str.to_lowercase();

                while let Some(start_in_lower) =
                    lower_line_content[current_search_pos..].find(&lower_query)
                {
                    let actual_start = current_search_pos + start_in_lower;
                    let match_len = lower_query.len();
                    let actual_end = actual_start + match_len;

                    print!("{}", &line_content_str[last_end..actual_start]);
                    print!(
                        "{}",
                        (&line_content_str[actual_start..actual_end]).red().bold()
                    );

                    last_end = actual_end;
                    current_search_pos = actual_end;
                    if current_search_pos >= line_content_str.len() {
                        break;
                    }
                }
                println!("{}", &line_content_str[last_end..]);
            }
        }
    }

    Ok(())
}

pub struct Config {
    pub query: String,
    pub file_name: String,
    pub case_sensitive: bool,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 3 {
            return Err("Looks like you passed some wonky doohickeys");
        }
        let query = args[1].clone();
        let file_name = args[2].clone();

        let case_sensitive = env::var("CASE_INSENSITIVE").is_err();

        Ok(Config {
            query,
            file_name,
            case_sensitive,
        })
    }
}

// Duplicated functions and struct (search, search_case_insensitive, analyze_structure, extract_keywords, StructuralAnalysisResults)
// are now removed and imported from text_analysis.rs
