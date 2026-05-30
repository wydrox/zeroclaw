//! Audio-safe formatting for spoken channel output.
//!
//! The formatter is intentionally deterministic and conservative: it removes
//! common Markdown that sounds bad in TTS and expands high-value symbols (for
//! now mostly weather/measurement output) into natural Polish speech.

use regex::Regex;
use std::sync::OnceLock;

fn ansi_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\x1b\[[0-9;]*m").expect("valid ANSI regex"))
}

fn fenced_code_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?s)```.*?```").expect("valid fenced code regex"))
}

fn temperature_c_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(-?\d+(?:[\.,]\d+)?)\s*(?:°\s*c|\*\s*c|c)\b")
            .expect("valid Celsius regex")
    })
}

fn percent_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(\d+(?:[\.,]\d+)?)\s*%").expect("valid percent regex"))
}

fn markdown_bullet_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^\s*[-*+]\s+").expect("valid markdown bullet regex"))
}

fn markdown_heading_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^\s{0,3}#{1,6}\s*").expect("valid markdown heading regex"))
}

fn whitespace_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s+").expect("valid whitespace regex"))
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let mut out: String = text.chars().take(keep).collect();
    out.push('…');
    out
}

/// Format text for spoken Polish TTS output.
///
/// This does not try to be a general natural-language rewriter. It handles the
/// high-signal transformations that are safe to do deterministically.
pub fn format_for_tts(text: &str, max_chars: Option<usize>) -> String {
    let mut out = text.to_string();
    out = ansi_re().replace_all(&out, "").into_owned();
    out = fenced_code_re()
        .replace_all(&out, " fragment kodu ")
        .into_owned();
    out = markdown_heading_re().replace_all(&out, "").into_owned();
    out = markdown_bullet_re().replace_all(&out, "").into_owned();
    out = temperature_c_re()
        .replace_all(&out, "$1 stopni Celsjusza")
        .into_owned();
    out = percent_re().replace_all(&out, "$1 procent").into_owned();

    // Remove common Markdown punctuation that tends to be spoken literally by TTS.
    for ch in ['`', '*', '_', '#', '>', '[', ']'] {
        out = out.replace(ch, "");
    }

    out = whitespace_re().replace_all(out.trim(), " ").into_owned();
    if let Some(max_chars) = max_chars {
        truncate_chars(&out, max_chars)
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::format_for_tts;

    #[test]
    fn expands_celsius_variants() {
        assert_eq!(
            format_for_tts("Jest 18°C, jutro 19 *C i potem 20 C.", None),
            "Jest 18 stopni Celsjusza, jutro 19 stopni Celsjusza i potem 20 stopni Celsjusza."
        );
    }

    #[test]
    fn strips_markdown_and_code_fences() {
        let formatted = format_for_tts(
            "## Pogoda\n- **Wiatr**: `silny`\n```bash\necho test\n```",
            None,
        );
        assert_eq!(formatted, "Pogoda Wiatr: silny fragment kodu");
    }

    #[test]
    fn expands_percent_and_truncates() {
        assert_eq!(format_for_tts("UV: 40%", None), "UV: 40 procent");
        assert_eq!(format_for_tts("abcdef", Some(4)), "abc…");
    }
}
