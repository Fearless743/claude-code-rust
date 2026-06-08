use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDecision {
    Allow,
    Deny(String),
    Ask,
}

#[derive(Debug, Clone)]
pub struct PermissionChecker {
    mode: crate::state::PermissionMode,
    allow_rules: Vec<String>,
    deny_rules: Vec<String>,
}

impl PermissionChecker {
    pub fn new(mode: crate::state::PermissionMode) -> Self {
        Self {
            mode,
            allow_rules: Vec::new(),
            deny_rules: Vec::new(),
        }
    }

    pub fn check(&self, tool_name: &str, _input: &serde_json::Value) -> PermissionDecision {
        match self.mode {
            crate::state::PermissionMode::BypassPermissions => PermissionDecision::Allow,
            crate::state::PermissionMode::Default => {
                // Check deny rules first, then allow rules, then ask
                for rule in &self.deny_rules {
                    if tool_name == rule.as_str() {
                        return PermissionDecision::Deny(format!("{tool_name} is denied by rules"));
                    }
                }
                for rule in &self.allow_rules {
                    if tool_name == rule.as_str() {
                        return PermissionDecision::Allow;
                    }
                }
                PermissionDecision::Ask
            }
            crate::state::PermissionMode::AcceptEdits => {
                if ["file_read", "file_write", "file_edit"].contains(&tool_name) {
                    PermissionDecision::Allow
                } else {
                    PermissionDecision::Ask
                }
            }
            crate::state::PermissionMode::Plan => {
                PermissionDecision::Deny("Plan mode: only read operations allowed".to_string())
            }
            crate::state::PermissionMode::Auto => PermissionDecision::Allow,
        }
    }
}
