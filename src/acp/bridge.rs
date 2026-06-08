use crate::api::message::{ContentBlock, Message};

pub fn acp_update_to_message(_session_id: &str, update: &super::agent::SessionUpdate) -> Message {
    let content: Vec<ContentBlock> = update
        .content
        .iter()
        .map(|c| match c {
            super::agent::UpdateContent::Text { text } => ContentBlock::Text { text: text.clone() },
            super::agent::UpdateContent::ToolUse { id, name, input } => ContentBlock::ToolUse {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            },
        })
        .collect();

    Message::Assistant {
        id: uuid::Uuid::new_v4(),
        content,
        model: String::new(),
        stop_reason: None,
        usage: None,
        timestamp: chrono::Utc::now(),
    }
}

pub fn message_to_acp_update(msg: &Message) -> Option<super::agent::SessionUpdate> {
    match msg {
        Message::Assistant { content, .. } => {
            let updates: Vec<super::agent::UpdateContent> = content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => {
                        Some(super::agent::UpdateContent::Text { text: text.clone() })
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        Some(super::agent::UpdateContent::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            input: input.clone(),
                        })
                    }
                    _ => None,
                })
                .collect();
            Some(super::agent::SessionUpdate {
                update_type: "message".into(),
                content: updates,
            })
        }
        _ => None,
    }
}
