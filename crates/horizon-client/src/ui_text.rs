//! Fit catalog descriptions in linear time, including kerning at the ellipsis.
pub fn fit(text: &str, width: f32, mut advance: impl FnMut(Option<char>, char) -> f32) -> String {
    let mut used = 0.0;
    let mut previous = None;
    let mut best = None;
    for (index, ch) in text.char_indices() {
        let mut candidate = used;
        let mut last = previous;
        for dot in ['.'; 3] {
            candidate += advance(last, dot);
            last = Some(dot);
        }
        if candidate <= width {
            best = Some(index);
        }
        used += advance(previous, ch);
        previous = Some(ch);
    }
    if used <= width {
        return text.to_owned();
    }
    best.map(|end| format!("{}...", &text[..end]))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_tail_trimming_with_unicode_and_kerning_without_quadratic_work() {
        let advance = |prev, ch| {
            if prev == Some('A') && ch == 'V' {
                -0.5
            } else if ch == '.' {
                0.5
            } else {
                1.0
            }
        };
        let measure = |s: &str| {
            let mut prev = None;
            s.chars()
                .map(|ch| {
                    let n = advance(prev, ch);
                    prev = Some(ch);
                    n
                })
                .sum::<f32>()
        };
        for text in ["", "a", "AVАБВ😀текст", "abcd", "...", "AVAVAV"] {
            for width in [-1.0, 0.0, 0.5, 1.5, 2.5, 4.0, 12.0] {
                let mut old = text.to_owned();
                if measure(&old) > width {
                    let mut answer = String::new();
                    while !old.is_empty() {
                        old.pop();
                        let candidate = format!("{old}...");
                        if measure(&candidate) <= width {
                            answer = candidate;
                            break;
                        }
                    }
                    old = answer;
                }
                assert_eq!(fit(text, width, advance), old, "{text}, {width}");
            }
        }
        let mut calls = 0;
        let text = "Описание 😀 ".repeat(5000);
        let result = fit(&text, 30.0, |p, c| {
            calls += 1;
            advance(p, c)
        });
        assert!(measure(&result) <= 30.0);
        assert_eq!(calls, text.chars().count() * 4);
    }
}
