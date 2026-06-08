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

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = chrono::Utc::now();
    }
}

pub struct SessionStore {
    base_dir: std::path::PathBuf,
}

impl SessionStore {
    pub fn new() -> eyre::Result<Self> {
        let dir = directories::ProjectDirs::from("com", "anthropic", "claude")
            .map(|d| d.data_dir().join("sessions"))
            .unwrap_or_else(|| std::path::PathBuf::from(".claude/sessions"));
        std::fs::create_dir_all(&dir)?;
        Ok(Self { base_dir: dir })
    }

    pub fn save(&self, session: &Session) -> eyre::Result<()> {
        let path = self.base_dir.join(format!("{}.json", session.id));
        let json = serde_json::to_string_pretty(session)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    pub fn load(&self, id: &Uuid) -> eyre::Result<Session> {
        let path = self.base_dir.join(format!("{id}.json"));
        let json = std::fs::read_to_string(&path)?;
        let session: Session = serde_json::from_str(&json)?;
        Ok(session)
    }

    pub fn list_sessions(&self) -> eyre::Result<Vec<Session>> {
        let mut sessions = Vec::new();
        for entry in std::fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            if entry.path().extension().map_or(false, |e| e == "json") {
                if let Ok(json) = std::fs::read_to_string(entry.path()) {
                    if let Ok(session) = serde_json::from_str::<Session>(&json) {
                        sessions.push(session);
                    }
                }
            }
        }
        sessions.sort_by_key(|s| s.updated_at);
        sessions.reverse();
        Ok(sessions)
    }

    pub fn delete(&self, id: &Uuid) -> eyre::Result<()> {
        let path = self.base_dir.join(format!("{id}.json"));
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = Session::new();
        assert!(session.messages.is_empty());
        assert!(session.title.is_none());
    }

    #[test]
    fn test_session_add_message() {
        let mut session = Session::new();
        let msg = Message::System {
            id: Uuid::new_v4(),
            content: "test".into(),
            timestamp: chrono::Utc::now(),
        };
        session.add_message(msg);
        assert_eq!(session.messages.len(), 1);
    }

    #[test]
    fn test_session_store_save_load() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = SessionStore {
            base_dir: dir.path().to_path_buf(),
        };

        let mut session = Session::new();
        session.title = Some("test session".into());
        store.save(&session).unwrap();

        let loaded = store.load(&session.id).unwrap();
        assert_eq!(loaded.title, Some("test session".into()));
    }
}
