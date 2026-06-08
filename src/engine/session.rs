use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::message::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub messages: Vec<Message>,
    pub title: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Session {
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            messages: Vec::new(),
            title: None,
            created_at: now,
            updated_at: now,
        }
    }
}
