use eyre::Result;

pub fn load_from_env() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .or_else(|| std::env::var("OPENAI_API_KEY").ok())
        .or_else(|| std::env::var("GEMINI_API_KEY").ok())
        .filter(|k| !k.is_empty())
}

pub fn save_to_config(_config_dir: &std::path::Path, _key: &str) -> Result<()> {
    todo!("Persist API key to config")
}
