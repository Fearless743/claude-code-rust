use eyre::Result;

#[derive(Debug, Clone)]
pub enum AuthCredentials {
    ApiKey(String),
    OAuth { access_token: String },
    None,
}

pub fn resolve_auth(api_key: Option<String>) -> AuthCredentials {
    if let Some(key) = api_key {
        if !key.is_empty() {
            return AuthCredentials::ApiKey(key);
        }
    }
    AuthCredentials::None
}

pub fn validate_api_key(key: &str) -> Result<()> {
    if key.starts_with("sk-ant-api03-") && key.len() >= 50 {
        Ok(())
    } else if key.starts_with("sk-") && key.len() >= 40 {
        Ok(())
    } else {
        Err(eyre::eyre!("Invalid API key format"))
    }
}
