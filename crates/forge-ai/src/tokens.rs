/// Deterministic token count estimator without requiring heavyweight external BPE tokenizer dependencies.
///
/// Computes token count via character-length heuristic (\~3.5 - 4 chars per token in English/code):
/// returns `ceil(char_count / 4)` with a minimum of 1 token for non-empty text.
#[must_use]
pub fn estimate_tokens(text: &str) -> u32 {
    let char_count = text.chars().count();
    if char_count == 0 {
        return 0;
    }
    ((char_count.saturating_add(3)) / 4) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_tokens_handles_empty_and_short_strings() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens("word"), 1);
        assert_eq!(estimate_tokens("hello world!"), 3);
    }

    #[test]
    fn estimate_tokens_handles_multibyte_characters() {
        assert_eq!(estimate_tokens("🦀🦀🦀🦀"), 1);
        assert_eq!(estimate_tokens("こんにちは世界"), 2);
    }
}
