use crate::app_structs::{SearchMatch, StructuralAnalysisResults};
use regex::RegexBuilder;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub regex_mode: bool,
    pub whole_word: bool,
}

pub fn search(query: &str, contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter(|line| line.contains(query))
        .map(|line| line.to_string())
        .collect()
}

pub fn search_case_insensitive(query: &str, contents: &str) -> Vec<String> {
    let query_lower = query.to_lowercase();
    contents
        .lines()
        .filter(|line| line.to_lowercase().contains(&query_lower))
        .map(|line| line.to_string())
        .collect()
}

pub fn search_with_options(
    query: &str,
    contents: &str,
    options: SearchOptions,
) -> Result<Vec<SearchMatch>, String> {
    if query.is_empty() {
        return Ok(Vec::new());
    }

    if !options.regex_mode && !options.whole_word {
        let matched_lines = if options.case_sensitive {
            search(query, contents)
        } else {
            search_case_insensitive(query, contents)
        };

        let mut remaining_matches = HashMap::new();
        for line in matched_lines {
            *remaining_matches.entry(line).or_insert(0usize) += 1;
        }

        return Ok(contents
            .lines()
            .enumerate()
            .filter_map(|(index, line)| {
                let remaining = remaining_matches.get_mut(line)?;
                if *remaining == 0 {
                    return None;
                }
                *remaining -= 1;

                let match_count = if options.case_sensitive {
                    line.match_indices(query).count()
                } else {
                    count_case_insensitive_matches(line, query)
                };

                Some(SearchMatch {
                    line_number: index + 1,
                    line: line.to_string(),
                    match_count,
                })
            })
            .collect());
    }

    let pattern = if options.regex_mode {
        if options.whole_word {
            format!(r"\b(?:{})\b", query)
        } else {
            query.to_string()
        }
    } else {
        let escaped = regex::escape(query);
        if options.whole_word {
            format!(r"\b{}\b", escaped)
        } else {
            escaped
        }
    };

    let regex = RegexBuilder::new(&pattern)
        .case_insensitive(!options.case_sensitive)
        .build()
        .map_err(|error| error.to_string())?;

    Ok(contents
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let match_count = regex.find_iter(line).count();
            if match_count == 0 {
                None
            } else {
                Some(SearchMatch {
                    line_number: index + 1,
                    line: line.to_string(),
                    match_count,
                })
            }
        })
        .collect())
}

pub fn analyze_structure(contents: &str) -> StructuralAnalysisResults {
    let lines = contents.lines().count();
    let words = contents.split_whitespace().count();
    let characters = contents.chars().count();
    let stanzas = contents.split("\n\n").count();
    let empty_lines = contents.lines().filter(|line| line.trim().is_empty()).count();
    let longest_line = contents.lines().map(|line| line.chars().count()).max().unwrap_or(0);

    let cleaned_words: Vec<String> = contents
        .split_whitespace()
        .map(normalize_word)
        .filter(|word| !word.is_empty())
        .collect();
    let unique_words = cleaned_words.iter().collect::<HashSet<_>>().len();
    let total_word_length: usize = cleaned_words.iter().map(|word| word.chars().count()).sum();
    let average_word_length = if cleaned_words.is_empty() {
        0.0
    } else {
        total_word_length as f64 / cleaned_words.len() as f64
    };

    StructuralAnalysisResults {
        lines,
        words,
        characters,
        stanzas,
        empty_lines,
        unique_words,
        longest_line,
        average_word_length,
    }
}

pub fn extract_keywords(contents: &str, top_n: usize) -> Vec<(String, usize)> {
    let mut word_counts = HashMap::new();
    for word in contents.split_whitespace() {
        let cleaned_word = normalize_word(word);
        if cleaned_word.len() >= 3 {
            *word_counts.entry(cleaned_word).or_insert(0) += 1;
        }
    }

    let mut sorted_keywords: Vec<(String, usize)> = word_counts.into_iter().collect();
    sorted_keywords.sort_by(|a, b| b.1.cmp(&a.1));
    sorted_keywords.truncate(top_n);
    sorted_keywords
}

pub fn extract_repeated_lines(contents: &str, top_n: usize) -> Vec<(String, usize)> {
    let mut counts = HashMap::new();

    for line in contents.lines().map(str::trim).filter(|line| !line.is_empty()) {
        *counts.entry(line.to_string()).or_insert(0) += 1;
    }

    let mut repeated: Vec<(String, usize)> = counts.into_iter().filter(|(_, count)| *count > 1).collect();
    repeated.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    repeated.truncate(top_n);
    repeated
}

fn normalize_word(word: &str) -> String {
    word.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
}

fn count_case_insensitive_matches(line: &str, query: &str) -> usize {
    let lower_line = line.to_lowercase();
    let lower_query = query.to_lowercase();
    let mut count = 0;
    let mut start = 0;

    while let Some(offset) = lower_line[start..].find(&lower_query) {
        count += 1;
        start += offset + lower_query.len();
    }

    count
}
