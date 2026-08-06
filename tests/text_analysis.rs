//! Keyword weighting, phrase extraction, language detection and readability.

use flerp::stopwords::Language;
use flerp::text_analysis::analyze_content;

/// Five paragraphs about two different things. "widget" runs through all of
/// them, so it is scaffolding; "kestrel" is confined to one, so it is a topic.
const MIXED: &str = "\
The widget report covers the kestrel programme and its outcomes.
Kestrel numbers rose sharply, and the kestrel survey confirmed the trend.

The widget report also covers the harbour extension works.
Harbour dredging continued through winter without incident.

The widget report notes the harbour extension works again.
Harbour traffic grew, and harbour revenue followed.

The widget report lists the staffing changes for the year.
Two managers joined and one retired.

The widget report closes with the budget summary for the year.
Spending fell against forecast.";

fn words(analysis: &flerp::text_analysis::ContentAnalysis) -> Vec<&str> {
    analysis
        .keywords
        .iter()
        .map(|keyword| keyword.word.as_str())
        .collect()
}

#[test]
fn stopwords_never_reach_the_keyword_list() {
    let analysis = analyze_content(MIXED, 20);
    let listed = words(&analysis);

    for stopword in ["the", "and", "its", "for", "with", "through", "also"] {
        assert!(
            !listed.contains(&stopword),
            "{stopword:?} should have been filtered, got {listed:?}"
        );
    }
    assert!(!listed.is_empty(), "filtering should not empty the list");
}

#[test]
fn a_word_in_every_section_is_weighted_below_a_word_in_one() {
    // A high limit on purpose: the point is where the two words sit relative to
    // each other, and the fixture has enough content words to push the
    // scaffolding one well down the list.
    let analysis = analyze_content(MIXED, 200);

    let widget = analysis
        .keywords
        .iter()
        .find(|keyword| keyword.word == "widget")
        .expect("widget should still be listed");
    let kestrel = analysis
        .keywords
        .iter()
        .find(|keyword| keyword.word == "kestrel")
        .expect("kestrel should be listed");

    // Raw frequency would put widget first. That is the behaviour being fixed.
    assert!(
        widget.count > kestrel.count,
        "the fixture needs widget to be the more frequent word"
    );
    assert!(
        kestrel.score > widget.score,
        "kestrel ({:.2}) should outweigh widget ({:.2}) despite being rarer",
        kestrel.score,
        widget.score
    );
}

#[test]
fn keywords_come_back_ordered_by_weight_and_capped() {
    let analysis = analyze_content(MIXED, 4);

    assert_eq!(analysis.keywords.len(), 4);
    for pair in analysis.keywords.windows(2) {
        assert!(
            pair[0].score >= pair[1].score,
            "keywords are not ordered by weight: {:?}",
            words(&analysis)
        );
    }
}

#[test]
fn repeated_phrases_are_found_and_one_offs_are_not() {
    let analysis = analyze_content(MIXED, 10);
    let phrases: Vec<&str> = analysis
        .phrases
        .iter()
        .map(|phrase| phrase.text.as_str())
        .collect();

    assert!(
        phrases.contains(&"widget report"),
        "expected the repeated phrase, got {phrases:?}"
    );
    assert!(
        phrases.contains(&"harbour extension works"),
        "expected the three-word repeat, got {phrases:?}"
    );
    assert!(
        !phrases.contains(&"budget summary"),
        "a phrase seen once is not a theme: {phrases:?}"
    );
    assert!(analysis.phrases.iter().all(|phrase| phrase.count > 1));
}

#[test]
fn phrases_do_not_run_through_stopwords_or_punctuation() {
    // "rose" and "and" sit between the repeated words on purpose.
    let text = "kestrel survey rose. kestrel survey rose. kestrel survey rose.";
    let analysis = analyze_content(text, 10);
    let phrases: Vec<&str> = analysis
        .phrases
        .iter()
        .map(|phrase| phrase.text.as_str())
        .collect();

    assert!(phrases.contains(&"kestrel survey rose"), "got {phrases:?}");
    assert!(
        !phrases.iter().any(|phrase| phrase.contains("rose kestrel")),
        "a phrase must not cross a full stop: {phrases:?}"
    );
}

#[test]
fn phrase_word_counts_match_the_text() {
    let analysis = analyze_content(MIXED, 10);

    for phrase in &analysis.phrases {
        assert_eq!(
            phrase.words,
            phrase.text.split_whitespace().count(),
            "word count disagrees with the phrase itself: {phrase:?}"
        );
        assert!(
            (2..=4).contains(&phrase.words),
            "{phrase:?} is out of range"
        );
    }
}

#[test]
fn english_and_turkish_are_told_apart() {
    let english = "\
This document describes a terminal tool that reads files and reports what is inside them.
The tool counts the words on every page and shows a summary of what it found.
If the file is large it only processes the part that is on the screen, so it stays fast.";

    let turkish = "\
Bu belge, terminal üzerinde çalışan bir metin analiz aracını anlatıyor.
Araç, dosyaları okur ve içindeki kelimeleri sayar. Kullanıcı isterse arama yapabilir.
Her sayfa için ayrı bir özet çıkarılır ve bu özetler ekranda gösterilir.";

    assert_eq!(analyze_content(english, 5).language, Language::English);
    assert_eq!(analyze_content(turkish, 5).language, Language::Turkish);
}

#[test]
fn a_repeated_table_heading_does_not_decide_the_language() {
    // Shaped like a real licence comparison: Turkish prose around a table whose
    // three English headings repeat down every row. Counting occurrences made
    // this English, because three words scored fifty-odd hits.
    let mut text = String::from(
        "Bu belge, açık kaynak lisanslarını karşılaştırmak için hazırlanmıştır. \
         Her lisans için ticari kullanım ve dağıtım koşulları ayrı ayrı gösterilir. \
         Kullanıcı bu tabloya bakarak kendi projesine uygun olanı seçebilir. \
         Ancak bazı lisanslar daha katıdır ve türev çalışmaların aynı lisansla \
         yayımlanmasını şart koşar; diğerleri ise sadece atıf ister. \
         Eğer projeniz kapalı kaynak olacaksa bu ayrımı çok dikkatli okuyun, \
         çünkü sonradan lisans değiştirmek hiç kolay değildir.\n",
    );
    for _ in 0..20 {
        text.push_str("can cannot must\n");
    }

    let analysis = analyze_content(&text, 10);

    assert_eq!(analysis.language, Language::Turkish);
    let listed = words(&analysis);
    assert!(
        !listed.contains(&"can") && !listed.contains(&"must"),
        "table headings should be filtered whatever the language guess: {listed:?}"
    );
}

#[test]
fn too_little_text_to_judge_is_reported_as_unknown() {
    // Guessing from a handful of words would be a coin toss dressed up as a
    // result, and the caller filters with both lists when told Unknown.
    assert_eq!(
        analyze_content("kestrel harbour widget", 5).language,
        Language::Unknown
    );
    assert_eq!(analyze_content("", 5).language, Language::Unknown);
}

#[test]
fn an_unknown_language_still_filters_with_both_lists() {
    // Word salad with no clear winner, but the stopwords in it are obvious.
    let text = "the kestrel ve harbour bir widget and dredging için survey";
    let analysis = analyze_content(text, 20);
    let listed = words(&analysis);

    assert_eq!(analysis.language, Language::Unknown);
    for stopword in ["the", "and", "bir", "için"] {
        assert!(
            !listed.contains(&stopword),
            "{stopword:?} survived an undetermined language: {listed:?}"
        );
    }
}

#[test]
fn sentences_are_counted_by_their_terminators() {
    let analysis = analyze_content("One thing. Two things! Three things? Wait!!!", 5);
    assert_eq!(
        analysis.readability.sentences, 4,
        "a run of terminators is one sentence, not three"
    );
}

#[test]
fn text_without_a_terminator_still_counts_as_one_sentence() {
    let analysis = analyze_content("a line with no full stop at the end of it", 5);

    assert_eq!(analysis.readability.sentences, 1);
    assert!(
        analysis.readability.words_per_sentence > 0.0,
        "dividing by zero sentences would make this meaningless"
    );
}

#[test]
fn short_plain_writing_scores_easier_than_long_dense_writing() {
    let plain = "The cat sat. The dog ran. The sun was up. We went out. It was fun.";
    let dense = "Notwithstanding the aforementioned considerations, the organisational \
                 restructuring initiative necessitated comprehensive reconsideration of \
                 established administrative methodologies throughout the participating \
                 departments and their respective subsidiaries.";

    let easy = analyze_content(plain, 5).readability;
    let hard = analyze_content(dense, 5).readability;

    assert!(
        easy.lix < hard.lix,
        "plain {:.1} should score below dense {:.1}",
        easy.lix,
        hard.lix
    );
    assert_eq!(easy.band, "very easy");
    assert_eq!(hard.band, "very difficult");
}

#[test]
fn long_word_share_is_a_fraction_of_the_whole_text() {
    let analysis = analyze_content(MIXED, 5);
    let share = analysis.readability.long_word_share;

    assert!(
        (0.0..=1.0).contains(&share),
        "share should be a fraction, got {share}"
    );
    assert!(share > 0.0, "this fixture has long words in it");
}

#[test]
fn empty_text_produces_an_empty_analysis_rather_than_a_panic() {
    let analysis = analyze_content("", 10);

    assert!(analysis.keywords.is_empty());
    assert!(analysis.phrases.is_empty());
    assert_eq!(analysis.readability.sentences, 0);
    assert_eq!(analysis.readability.lix, 0.0);
    assert_eq!(analysis.readability.band, "no text");
}

#[test]
fn a_single_paragraph_falls_back_to_lines_for_weighting() {
    // No blank line, so paragraphs would give one section and idf nothing to
    // work with. Lines still vary, so the confined word should still win.
    let text = "\
widget kestrel kestrel kestrel
widget harbour
widget staffing
widget budget";

    let analysis = analyze_content(text, 10);
    let kestrel = analysis
        .keywords
        .iter()
        .find(|keyword| keyword.word == "kestrel")
        .expect("kestrel should be listed");
    let widget = analysis
        .keywords
        .iter()
        .find(|keyword| keyword.word == "widget")
        .expect("widget should be listed");

    assert!(
        kestrel.score > widget.score,
        "kestrel {:.2} should outweigh widget {:.2}",
        kestrel.score,
        widget.score
    );
}
