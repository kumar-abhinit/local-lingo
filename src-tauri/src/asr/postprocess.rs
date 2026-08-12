/// Light post-processing for MVP — no LLM rewriting.

const SPOKEN_COMMANDS: &[(&str, &str)] = &[
    ("new line", "\n"),
    ("newline", "\n"),
    ("comma", ", "),
    ("period", ". "),
    ("full stop", ". "),
    ("question mark", "? "),
    ("exclamation mark", "! "),
    ("exclamation point", "! "),
];

pub fn postprocess(raw: &str) -> String {
    let mut text = raw.trim().to_string();
    if text.is_empty() {
        return text;
    }

    for (spoken, replacement) in SPOKEN_COMMANDS {
        text = replace_spoken_phrase(&text, spoken, replacement);
    }

    text = collapse_whitespace(&text);
    text = capitalize_first_char(&text);
    text = ensure_terminal_punctuation(&text);
    text
}

fn replace_spoken_phrase(text: &str, spoken: &str, replacement: &str) -> String {
    let lower = text.to_lowercase();
    let mut result = text.to_string();
    if let Some(idx) = lower.find(spoken) {
        let end = idx + spoken.len();
        result.replace_range(idx..end, replacement);
    }
    result
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn capitalize_first_char(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn ensure_terminal_punctuation(text: &str) -> String {
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        return text.to_string();
    }
    if trimmed.ends_with(['.', '!', '?', ',', ':', ';', '\n']) {
        return trimmed.to_string();
    }
    format!("{trimmed}.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitalizes_and_adds_period() {
        assert_eq!(postprocess("hello world"), "Hello world.");
    }

    #[test]
    fn spoken_newline() {
        assert_eq!(postprocess("first new line second"), "First\nSecond.");
    }
}
