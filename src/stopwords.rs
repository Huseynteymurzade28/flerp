//! Stopword lists, and the language guess reported alongside them.
//!
//! Frequency alone makes every document look the same: `the`, `and`, `of` win
//! in English and `bir`, `ve`, `için` win in Turkish, so the keyword panel ends
//! up describing the language rather than the text. Removing these is what
//! makes the rest of the analysis mean anything.

use serde::Serialize;
use std::collections::HashSet;

/// The languages flerp can tell apart well enough to pick a stopword list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    English,
    Turkish,
    /// Neither list matched enough to be worth trusting.
    #[default]
    Unknown,
}

impl Language {
    pub fn label(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Turkish => "Turkish",
            Language::Unknown => "Undetermined",
        }
    }
}

#[rustfmt::skip]
const ENGLISH: &[&str] = &[
    "a", "about", "above", "after", "again", "against", "all", "also", "always", "am", "among",
    "an", "and", "another", "any", "are", "around", "as", "at", "be", "because", "become",
    "becomes", "been", "before", "being", "below", "between", "both", "but", "by", "came",
    "can", "cannot", "come", "comes", "could", "did", "do", "does", "doing", "down", "during",
    "each", "even", "ever", "every", "few", "for", "from", "further", "get", "gets", "go",
    "goes", "got", "had", "has", "have", "having", "he", "her", "here", "hers", "herself",
    "him", "himself", "his", "how", "i", "if", "in", "into", "is", "it", "its", "itself",
    "just", "let", "like", "made", "make", "makes", "many", "may", "me", "might", "more",
    "most", "much", "must", "my", "myself", "need", "needs", "never", "no", "nor", "not", "now",
    "of", "off", "often", "on", "once", "one", "only", "onto", "or", "other", "ought", "our",
    "ours", "ourselves", "out", "over", "own", "per", "perhaps", "put", "rather", "really",
    "said", "same", "say", "says", "see", "seen", "she", "should", "since", "so", "some",
    "still", "such", "take", "takes", "than", "that", "the", "their", "theirs", "them",
    "themselves", "then", "there", "these", "they", "this", "those", "through", "thus", "to",
    "too", "under", "until", "up", "upon", "us", "use", "used", "using", "very", "want", "was",
    "way", "we", "well", "went", "were", "what", "when", "where", "whether", "which", "while",
    "who", "whom", "why", "will", "with", "within", "without", "would", "yet", "you", "your",
    "yours", "yourself", "yourselves",
];

#[rustfmt::skip]
const TURKISH: &[&str] = &[
    "acaba", "altı", "ama", "ancak", "artık", "asla", "aslında", "ayrıca", "az", "bana",
    "bazen", "bazı", "belki", "ben", "benden", "beni", "benim", "beş", "bile", "bir", "biraz",
    "biri", "birkaç", "birçok", "birşey", "biz", "bizden", "bize", "bizi", "bizim", "bu",
    "buna", "bunda", "bundan", "bunlar", "bunları", "bunların", "bunu", "bunun", "burada",
    "böyle", "böylece", "da", "daha", "de", "değil", "diye", "diğer", "dokuz", "dolayı", "dört",
    "elbette", "en", "eğer", "fakat", "gibi", "hangi", "hani", "hem", "henüz", "hep", "hepsi",
    "her", "herkes", "hiç", "hiçbir", "iki", "ile", "ise", "için", "içinde", "işte", "kadar",
    "karşın", "kendi", "kendine", "kez", "ki", "kim", "kimse", "mi", "mu", "mü", "mı", "nasıl",
    "ne", "neden", "nerede", "nereye", "niye", "niçin", "o", "olarak", "olduğu", "olsa",
    "olsun", "on", "ona", "ondan", "onlar", "onlardan", "onları", "onların", "onu", "onun",
    "orada", "pek", "rağmen", "sadece", "sana", "sekiz", "sen", "senden", "seni", "senin",
    "siz", "sizden", "size", "sizi", "sizin", "sonra", "tabii", "tüm", "var", "ve", "veya",
    "ya", "yani", "yedi", "yine", "yoksa", "zaten", "çok", "çünkü", "öyle", "üzere", "üç",
    "şey", "şimdi", "şu", "şuna", "şunu", "şöyle",
];

/// Words worth ignoring whatever the language is: they carry no topic in either
/// list's language and appear in mixed-language documents constantly.
const UNIVERSAL: &[&str] = &["com", "http", "https", "net", "org", "pdf", "www"];

/// Distinct stopwords a language needs before the guess is worth making.
const MIN_DISTINCT: usize = 5;

/// Guess the language from the variety of function words the text uses.
///
/// What counts is how many *different* stopwords appear, not how often. Totals
/// are easy to fool: a licence comparison table that repeats the headings
/// "can", "cannot" and "must" down every row scores 157 English hits from three
/// words, and would drag a Turkish document into the English list. Prose in a
/// language reaches for a wide spread of its function words; a table reaches for
/// the same few.
///
/// A weak or close result is reported as [`Language::Unknown`], which makes the
/// caller filter with both lists instead of confidently using the wrong one.
pub fn detect(words: &[String]) -> Language {
    let mut english = HashSet::new();
    let mut turkish = HashSet::new();

    for word in words {
        if in_list(ENGLISH, word) {
            english.insert(word.as_str());
        }
        if in_list(TURKISH, word) {
            turkish.insert(word.as_str());
        }
    }

    let (english, turkish) = (english.len(), turkish.len());
    let (winner, loser, language) = if english >= turkish {
        (english, turkish, Language::English)
    } else {
        (turkish, english, Language::Turkish)
    };

    // Too little variety to judge, or a margin narrow enough to be a coin toss.
    if winner < MIN_DISTINCT || winner < loser * 2 {
        return Language::Unknown;
    }

    language
}

/// True when the word carries no topic and should be kept out of the analysis.
///
/// Every list applies, whatever [`detect`] concluded. Documents mix languages
/// constantly — a Turkish page describing English licence terms, a report with
/// English headings — and filtering by the detected language alone lets the
/// other language's function words to the top of the keyword panel. Detection
/// therefore reports what the text is, and does not decide what gets filtered,
/// so a wrong guess costs nothing.
///
/// The price is the handful of words that are a stopword in one language and
/// content in the other: `var` and `en` are Turkish function words and could
/// matter in an English text about code. Losing those is a far smaller cost
/// than a panel full of `can`, `must` and `bir`.
pub fn is_stopword(word: &str) -> bool {
    in_list(UNIVERSAL, word) || in_list(ENGLISH, word) || in_list(TURKISH, word)
}

fn in_list(list: &[&str], word: &str) -> bool {
    list.binary_search(&word).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `in_list` binary searches, so an unsorted list would silently miss words.
    #[test]
    fn every_list_is_sorted_and_free_of_duplicates() {
        for (name, list) in [
            ("english", ENGLISH),
            ("turkish", TURKISH),
            ("universal", UNIVERSAL),
        ] {
            for pair in list.windows(2) {
                assert!(
                    pair[0] < pair[1],
                    "{name} list is out of order at {:?} / {:?}",
                    pair[0],
                    pair[1]
                );
            }
        }
    }
}
