//! Translate full IME buffers into the shared text editor's Unicode operations.
use crate::osk::Output;

/// Android supplies complete editor buffers. A newline confirms a single-line
/// editor, but must remain ordinary text when writing a book page.
pub fn editor_changes(old: &str, new: &str, multiline: bool) -> Vec<Output> {
    if old == new {
        return Vec::new();
    }
    let mut out = changes(old, new);
    if !multiline && new.ends_with('\n') {
        if let Some(Output::Text(text)) = out.last_mut() {
            text.pop();
        }
        out.push(Output::Enter);
    }
    out
}

pub fn changes(old: &str, new: &str) -> Vec<Output> {
    let prefix = old
        .chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .count();
    let mut out: Vec<_> = (prefix..old.chars().count())
        .map(|_| Output::Backspace)
        .collect();
    let tail: String = new.chars().skip(prefix).collect();
    if !tail.is_empty() {
        out.push(Output::Text(tail));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enter_confirms_fields_but_keeps_book_line_breaks() {
        let field = editor_changes("строка", "строка\n", false);
        assert!(matches!(field.last(), Some(Output::Enter)));
        assert!(
            !field
                .iter()
                .any(|op| matches!(op, Output::Text(t) if t.contains('\n')))
        );
        let page = editor_changes("строка", "строка\n", true);
        assert!(matches!(page.last(), Some(Output::Text(t)) if t == "\n"));
        assert!(!page.iter().any(|op| matches!(op, Output::Enter)));
        assert!(editor_changes("строка\n", "строка\n", false).is_empty());
    }
    #[test]
    fn unicode_composition_replacement_and_deletion() {
        for (old, new) in [
            ("привет", "привёт🙂"),
            ("abc🙂", "ab"),
            ("teh", "the"),
            ("", "你好"),
        ] {
            let mut text = old.to_owned();
            for op in changes(old, new) {
                match op {
                    Output::Backspace => {
                        text.pop();
                    }
                    Output::Text(s) => text.push_str(&s),
                    _ => unreachable!(),
                }
            }
            assert_eq!(text, new);
        }
    }
}
