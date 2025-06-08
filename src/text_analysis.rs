use crate::app_structs::StructuralAnalysisResults;
use std::collections::HashMap;

pub fn search<'a>(query: &str, contents: &'a str) -> Vec<String> {
    contents
        .lines()
        .filter(|line| line.contains(query))
        .map(|line| line.to_string())
        .collect()
}

pub fn search_case_insensitive<'a>(query: &str, contents: &'a str) -> Vec<String> {
    let query_lower = query.to_lowercase();
    contents
        .lines()
        .filter(|line| line.to_lowercase().contains(&query_lower))
        .map(|line| line.to_string())
        .collect()
}

pub fn analyze_structure(contents: &str) -> StructuralAnalysisResults {
    let lines = contents.lines().count();
    let words = contents.split_whitespace().count();
    let characters = contents.chars().count();
    let stanzas = contents.split("\n\n").count();

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
