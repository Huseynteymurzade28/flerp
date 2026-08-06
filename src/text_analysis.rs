use crate::app_structs::{SearchMatch, StructuralAnalysisResults};
use crate::stopwords::{self, Language};
use regex::RegexBuilder;
use serde::Serialize;
use std::cmp::Ordering;
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

/// Shortest word worth treating as a topic.
const MIN_KEYWORD_CHARS: usize = 3;
/// Longest phrase reported, in words.
const MAX_PHRASE_WORDS: usize = 4;
/// Words above this length are what LIX counts as hard going.
const LONG_WORD_CHARS: usize = 6;

/// A word the document is about, and the weight that put it there.
#[derive(Debug, Clone, Serialize)]
pub struct Keyword {
    pub word: String,
    pub count: usize,
    /// tf-idf against the document's own sections. Not comparable between
    /// files, only between words in the same file.
    pub score: f64,
}

/// A repeated run of content words, which usually reads better than the words
/// on their own: "masaj kremi" says more than "masaj" and "kremi" apart.
#[derive(Debug, Clone, Serialize)]
pub struct Phrase {
    pub text: String,
    pub count: usize,
    pub words: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Readability {
    pub language: Language,
    pub sentences: usize,
    pub words_per_sentence: f64,
    pub long_word_share: f64,
    /// LIX. Chosen over Flesch because it needs no syllable counting, which is
    /// English-only, and this tool is routinely pointed at Turkish.
    pub lix: f64,
    pub band: &'static str,
}

impl Default for Readability {
    fn default() -> Self {
        Self {
            language: Language::Unknown,
            sentences: 0,
            words_per_sentence: 0.0,
            long_word_share: 0.0,
            lix: 0.0,
            band: "no text",
        }
    }
}

/// Everything that depends on what the words mean rather than how many there
/// are.
#[derive(Debug, Clone, Default)]
pub struct ContentAnalysis {
    pub language: Language,
    pub keywords: Vec<Keyword>,
    pub phrases: Vec<Phrase>,
    pub readability: Readability,
}

/// Run the keyword, phrase and readability passes together.
///
/// They share one tokenising pass and one language guess. The guess is reported
/// rather than acted on: filtering uses every stopword list regardless, so a
/// mixed-language document is handled the same way whatever the guess was.
pub fn analyze_content(contents: &str, keyword_limit: usize) -> ContentAnalysis {
    let words: Vec<String> = contents
        .split_whitespace()
        .map(normalize_word)
        .filter(|word| !word.is_empty())
        .collect();
    let language = stopwords::detect(&words);

    ContentAnalysis {
        language,
        keywords: rank_keywords(&sections(contents), keyword_limit),
        phrases: rank_phrases(contents, keyword_limit),
        readability: readability(contents, &words, language),
    }
}

/// Split the text into the units tf-idf treats as separate documents.
///
/// Paragraphs are the natural unit: a term in every paragraph is scaffolding, a
/// term concentrated in a few is what the text is about. A file with no blank
/// lines collapses to a single paragraph, which tells idf nothing, so those
/// fall back to lines.
fn sections(contents: &str) -> Vec<&str> {
    let paragraphs: Vec<&str> = contents
        .split("\n\n")
        .map(str::trim)
        .filter(|section| !section.is_empty())
        .collect();

    if paragraphs.len() > 1 {
        return paragraphs;
    }

    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
}

/// Rank words by tf-idf over the document's own sections.
///
/// Raw frequency ranks every document the same way, and without a corpus there
/// is no external idf to borrow. Treating each section as a document gives one
/// for free: a word spread evenly through the text is scored down however often
/// it repeats, and a word that clusters is scored up.
fn rank_keywords(sections: &[&str], limit: usize) -> Vec<Keyword> {
    let mut totals: HashMap<String, usize> = HashMap::new();
    let mut section_counts: HashMap<String, usize> = HashMap::new();

    for section in sections {
        let mut seen = HashSet::new();
        for token in section.split_whitespace() {
            let word = normalize_word(token);
            if word.chars().count() < MIN_KEYWORD_CHARS || stopwords::is_stopword(&word) {
                continue;
            }

            *totals.entry(word.clone()).or_insert(0) += 1;
            if seen.insert(word.clone()) {
                *section_counts.entry(word).or_insert(0) += 1;
            }
        }
    }

    let total_sections = sections.len().max(1) as f64;
    let mut ranked: Vec<Keyword> = totals
        .into_iter()
        .map(|(word, count)| {
            let in_sections = section_counts.get(&word).copied().unwrap_or(1) as f64;
            // BM25's smoothed idf, which stays positive even for a word present
            // in every section. The textbook ln(N/df) goes to zero there and
            // negative just past it, which would sort common words below rare
            // ones by accident rather than by weight.
            let idf = (1.0 + (total_sections - in_sections + 0.5) / (in_sections + 0.5)).ln();
            Keyword {
                score: count as f64 * idf,
                word,
                count,
            }
        })
        .collect();

    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.word.cmp(&b.word))
    });
    ranked.truncate(limit);
    ranked
}

/// Pull out repeated multi-word phrases.
///
/// This is RAKE's idea: a phrase is a run of content words with no stopword or
/// punctuation breaking it. It needs no corpus and no grammar, only the
/// stopword list the keyword pass already uses.
fn rank_phrases(contents: &str, limit: usize) -> Vec<Phrase> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut run: Vec<String> = Vec::new();

    for token in contents.split_whitespace() {
        let word = normalize_word(token);
        if word.chars().count() < MIN_KEYWORD_CHARS || stopwords::is_stopword(&word) {
            flush_run(&mut run, &mut counts);
            continue;
        }

        let ends_here = token.chars().any(is_phrase_boundary);
        run.push(word);
        if ends_here {
            flush_run(&mut run, &mut counts);
        }
    }
    flush_run(&mut run, &mut counts);

    // A phrase seen once is a sentence, not a theme.
    let mut candidates: Vec<Phrase> = counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(text, count)| Phrase {
            words: text.split(' ').count(),
            text,
            count,
        })
        .collect();

    // Longest first, so a phrase is always judged against the longer ones that
    // might contain it.
    candidates.sort_by(|a, b| {
        b.words
            .cmp(&a.words)
            .then_with(|| b.count.cmp(&a.count))
            .then_with(|| a.text.cmp(&b.text))
    });

    let mut ranked: Vec<Phrase> = Vec::new();
    for candidate in candidates {
        // Every occurrence of "harbour extension works" is also an occurrence
        // of "harbour extension", so listing both says the same thing twice.
        // Only the shorter one earns its place if it also turns up somewhere
        // the longer one does not.
        let subsumed = ranked
            .iter()
            .any(|kept| kept.count == candidate.count && contains_phrase(&kept.text, &candidate.text));
        if !subsumed {
            ranked.push(candidate);
        }
    }

    ranked.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| b.words.cmp(&a.words))
            .then_with(|| a.text.cmp(&b.text))
    });
    ranked.truncate(limit);
    ranked
}

/// True when `inner` appears in `outer` on whole-word boundaries.
fn contains_phrase(outer: &str, inner: &str) -> bool {
    format!(" {outer} ").contains(&format!(" {inner} "))
}

/// Record every phrase a finished run contains, and start a new run.
///
/// Sub-phrases count too, not only the whole run: "widget report" is the theme
/// even when a different verb follows it every time, so counting maximal runs
/// alone would miss it entirely. The redundancy that creates is filtered out
/// afterwards by dropping any phrase a longer one already accounts for.
fn flush_run(run: &mut Vec<String>, counts: &mut HashMap<String, usize>) {
    for size in 2..=run.len().min(MAX_PHRASE_WORDS) {
        for window in run.windows(size) {
            *counts.entry(window.join(" ")).or_insert(0) += 1;
        }
    }

    run.clear();
}

fn is_phrase_boundary(c: char) -> bool {
    matches!(
        c,
        '.' | ',' | ';' | ':' | '!' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\''
            | '«' | '»' | '—' | '–' | '/' | '|'
    )
}

fn readability(contents: &str, words: &[String], language: Language) -> Readability {
    if words.is_empty() {
        return Readability {
            language,
            ..Readability::default()
        };
    }

    let sentences = count_sentences(contents);
    let long = words
        .iter()
        .filter(|word| word.chars().count() > LONG_WORD_CHARS)
        .count();

    let words_per_sentence = words.len() as f64 / sentences as f64;
    let long_word_share = long as f64 / words.len() as f64;
    let lix = words_per_sentence + 100.0 * long_word_share;

    Readability {
        language,
        sentences,
        words_per_sentence,
        long_word_share,
        lix,
        band: lix_band(lix),
    }
}

/// The published LIX bands.
fn lix_band(lix: f64) -> &'static str {
    match lix {
        value if value < 25.0 => "very easy",
        value if value < 35.0 => "easy",
        value if value < 45.0 => "medium",
        value if value < 55.0 => "difficult",
        _ => "very difficult",
    }
}

/// Count sentences by their terminators.
///
/// A run of terminators counts once, so "Wait!!!" is one sentence. Abbreviations
/// still fool it; fixing that needs a per-language abbreviation list, and the
/// index this feeds is not precise enough to earn one. Text with no terminator
/// at all is one sentence, not zero, so the division below stays meaningful.
fn count_sentences(contents: &str) -> usize {
    let mut count = 0;
    let mut inside = false;

    for c in contents.chars() {
        if matches!(c, '.' | '!' | '?' | '…') {
            if !inside {
                count += 1;
            }
            inside = true;
        } else {
            inside = false;
        }
    }

    count.max(1)
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
