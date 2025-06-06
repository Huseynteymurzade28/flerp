use colored::*;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs; // Added for colored output

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    println!("  .--.  ");
    println!(" /  _ \\ ");
    println!("|  o o |");
    println!(" \\  ^ / ");
    println!("  `--'  ");
    println!("   ||   ");
    println!("        ");

    println!("    _________________________");
    println!("   /                        //");
    println!("  /   Flerp Text Analysis  //");
    println!(" /    Content & Insights  //");
    println!("/________________________//");
    println!("(________________________(/");
    println!("   \\                      \\");
    println!("    \\   Keywords, Stats,    \\");
    println!("     \\   Search Results...  \\");
    println!("      \\______________________\\");
    println!("");

    let contents = if config.file_name.ends_with(".pdf") {
        println!("{}", "Reading pdf file...".bright_blue().bold());
        pdf_extract::extract_text(&config.file_name)?
    } else {
        fs::read_to_string(config.file_name)?
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
    ); // Added character count
    println!(
        "  Stanzas: {}",
        structural_analysis_results.stanzas.to_string().yellow()
    );

    let keywords = extract_keywords(&contents, 5); // Extract top 5 keywords
    println!("\n{}", "Keywords (Top 5):".cyan().bold());
    for (keyword, count) in keywords {
        println!("  {}: {}", keyword.green().bold(), count.to_string().blue());
    }

    println!("\n{}", "Search Results:".cyan().bold());
    let search_results = if config.case_sensitive {
        search(&config.query, &contents)
    } else {
        search_case_insensitive(&config.query, &contents)
    };

    if search_results.is_empty() {
        println!("{}", "  No matches found.".italic());
    } else {
        for line_content in search_results {
            let query_to_highlight = &config.query;
            if config.case_sensitive {
                let mut last_end = 0;
                for (start, part) in line_content.match_indices(query_to_highlight) {
                    print!("{}", &line_content[last_end..start]);
                    print!("{}", part.magenta().bold());
                    last_end = start + part.len();
                }
                println!("{}", &line_content[last_end..]);
            } else {
                // Case-insensitive highlighting
                let lower_query = query_to_highlight.to_lowercase();
                let mut last_end = 0;
                let mut current_search_pos = 0;
                let lower_line_content = line_content.to_lowercase();

                while let Some(start_in_lower) =
                    lower_line_content[current_search_pos..].find(&lower_query)
                {
                    let actual_start = current_search_pos + start_in_lower;
                    let match_len = lower_query.len();
                    let actual_end = actual_start + match_len;

                    print!("{}", &line_content[last_end..actual_start]);
                    print!("{}", (&line_content[actual_start..actual_end]).red().bold());

                    last_end = actual_end;
                    current_search_pos = actual_end;
                    if current_search_pos >= line_content.len() {
                        break;
                    }
                }
                println!("{}", &line_content[last_end..]);
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
    pub fn new(args: &[String]) -> Result<Config, &str> {
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

pub fn search<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    let mut res = Vec::new();

    for line in contents.lines() {
        if line.contains(query) {
            res.push(line);
        }
    }
    res
}

pub fn search_case_insensitive<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    let query_lower = query.to_lowercase(); // Renamed for clarity
    let mut res = Vec::new();

    for line in contents.lines() {
        if line.to_lowercase().contains(&query_lower) {
            res.push(line);
        }
    }

    res
}

pub struct StructuralAnalysisResults {
    pub lines: usize,
    pub words: usize,
    pub characters: usize, // Added characters
    pub stanzas: usize,
}

// New public function for structural analysis
pub fn analyze_structure(contents: &str) -> StructuralAnalysisResults {
    let lines = contents.lines().count();
    let words = contents.split_whitespace().count();
    let characters = contents.chars().count(); // Calculate characters
    let stanzas = contents.split("\\n\\n").count(); // Note: Rust string literals need double backslash for newline

    StructuralAnalysisResults {
        lines,
        words,
        characters,
        stanzas,
    }
}

pub fn extract_keywords(contents: &str, top_n: usize) -> Vec<(String, usize)> {
    let mut word_counts = HashMap::new();
    for word in contents.split_whitespace() {
        let cleaned_word = word
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>();
        if !cleaned_word.is_empty() {
            *word_counts.entry(cleaned_word).or_insert(0) += 1;
        }
    }

    let mut sorted_keywords: Vec<(String, usize)> = word_counts.into_iter().collect();
    sorted_keywords.sort_by(|a, b| b.1.cmp(&a.1));
    sorted_keywords.truncate(top_n);
    sorted_keywords
}
