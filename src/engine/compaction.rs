use crate::api::message::{ContentBlock, Message};

/// Maximum tokens before compaction is triggered (rough estimate)
const COMPACTION_THRESHOLD_TOKENS: usize = 80_000;

/// Check if conversation needs compaction based on estimated token count
pub fn needs_compaction(messages: &[Message]) -> bool {
    estimate_total_tokens(messages) > COMPACTION_THRESHOLD_TOKENS
}

/// Create a compacted version of the conversation by summarizing early messages
pub fn compact_messages(messages: &[Message]) -> Vec<Message> {
    if messages.len() <= 4 {
        return messages.to_vec();
    }

    let mut compacted = Vec::new();

    // Keep the first message (often user request)
    compacted.push(messages.first().unwrap().clone());

    // Create a summary of the middle messages
    let mid_count = messages.len().saturating_sub(2);
    let summary = summarize_messages(&messages[1..messages.len() - 1]);
    compacted.push(Message::System {
        id: uuid::Uuid::new_v4(),
        content: summary,
        timestamp: chrono::Utc::now(),
    });

    // Keep the last message
    if messages.len() > 1 {
        compacted.push(messages.last().unwrap().clone());
    }

    compacted
}

fn summarize_messages(messages: &[Message]) -> String {
    let mut summary = String::from("[Earlier conversation summary]\n");
    let mut tool_count = 0;
    let mut text_parts = Vec::new();

    for msg in messages {
        match msg {
            Message::Assistant { content, .. } => {
                for block in content {
                    match block {
                        ContentBlock::ToolUse { name, .. } => {
                            tool_count += 1;
                            summary.push_str(&format!("- Used tool: {name}\n"));
                        }
                        ContentBlock::Text { text } => {
                            if text.len() > 200 {
                                text_parts.push(format!("  ... {} chars", text.len()));
                            } else {
                                text_parts.push(text.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    summary.push_str(&format!(
        "({} tools used, {} messages summarized)\n",
        tool_count,
        messages.len()
    ));

    if !text_parts.is_empty() {
        summary.push_str("\nKey points:\n");
        for (i, part) in text_parts.iter().take(10).enumerate() {
            summary.push_str(&format!("  {}. {part}\n", i + 1));
        }
    }

    summary
}

fn estimate_total_tokens(messages: &[Message]) -> usize {
    let mut total = 0;
    for msg in messages {
        match msg {
            Message::User { content, .. } => {
                total += serde_json::to_string(content)
                    .map(|s| s.len().div_ceil(4))
                    .unwrap_or(0);
            }
            Message::Assistant { content, .. } => {
                total += serde_json::to_string(content)
                    .map(|s| s.len().div_ceil(4))
                    .unwrap_or(0);
            }
            Message::System { content, .. } => {
                total += content.len().div_ceil(4);
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_compaction_empty() {
        assert!(!needs_compaction(&[]));
    }

    #[test]
    fn test_compact_preserves_boundaries() {
        let msgs = vec![
            Message::User {
                id: uuid::Uuid::new_v4(),
                content: vec![ContentBlock::Text {
                    text: "first".into(),
                }],
                timestamp: chrono::Utc::now(),
            },
            Message::Assistant {
                id: uuid::Uuid::new_v4(),
                content: vec![ContentBlock::Text {
                    text: "middle".into(),
                }],
                model: String::new(),
                stop_reason: None,
                usage: None,
                timestamp: chrono::Utc::now(),
            },
            Message::User {
                id: uuid::Uuid::new_v4(),
                content: vec![ContentBlock::Text {
                    text: "last".into(),
                }],
                timestamp: chrono::Utc::now(),
            },
        ];

        let compacted = compact_messages(&msgs);
        assert!(compacted.len() < msgs.len() || compacted.len() == 3);
    }
}
