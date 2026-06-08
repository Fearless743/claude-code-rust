use eyre::Result;

pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn start_oauth_flow() -> Result<OAuthToken> {
    todo!("OAuth login flow")
}

pub async fn refresh_token(token: &OAuthToken) -> Result<OAuthToken> {
    todo!("OAuth token refresh")
}
