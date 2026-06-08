pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

pub fn estimate_message_tokens(content: &str) -> usize {
    estimate_tokens(content)
}
