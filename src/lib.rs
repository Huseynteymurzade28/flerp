use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(config.file_name)?;

    let structural_analysis_results = analyze_structure(&contents);
    println!("Structural Analysis:");
    println!("  Lines: {}", structural_analysis_results.lines);
    println!("  Words: {}", structural_analysis_results.words);
    println!("  Stanzas: {}", structural_analysis_results.stanzas);

    let keywords = extract_keywords(&contents, 5); // Extract top 5 keywords
    println!("\nKeywords (Top 5):");
    for (keyword, count) in keywords {
        println!("  {}: {}", keyword, count);
    }

    let res = if config.case_sensitive {
        search(&config.query, &contents)
    } else {
        search_case_insensitive(&config.query, &contents)
    };

    for line in res {
        println!("{}", line);
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
    let query = query.to_lowercase();
    let mut res = Vec::new();

    for line in contents.lines() {
        if line.to_lowercase().contains(&query) {
            res.push(line);
        }
    }

    res
}

pub struct StructuralAnalysisResults {
    pub lines: usize,
    pub words: usize,
    pub stanzas: usize,
}

// New public function for structural analysis
pub fn analyze_structure(contents: &str) -> StructuralAnalysisResults {
    let lines = contents.lines().count();
    let words = contents.split_whitespace().count();
    let stanzas = contents.split("\n\n").count(); 

    StructuralAnalysisResults {
        lines,
        words,
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
