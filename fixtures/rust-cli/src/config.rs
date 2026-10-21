pub struct Config {
    pub app_name: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_name: "rust-cli".to_string(),
        }
    }
}
