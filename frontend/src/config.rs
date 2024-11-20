pub struct Config {
    pub api_base_url: &'static str,
}

impl Config {
    pub const fn new() -> Self {
        Self {
            api_base_url: "/api"
        }
    }
}

pub const CONFIG: Config = Config::new();