use std::{path::PathBuf, fs::{create_dir_all, File}, io::BufReader};

use anyhow::Context;
use poise::serenity_prelude::ChannelId;
use tracing::info;

use serde::Deserialize;
#[derive(Deserialize)]
struct Config {
    main: MainConfig,
    secret_lab: SLConfig,
    discord: DiscordConfig,
}
#[derive(Deserialize)]
struct MainConfig {
    token: String
}
#[derive(Deserialize)]
struct SLConfig {
    audit: ChannelId
}
#[derive(Deserialize)]
struct DiscordConfig {
    audit: ChannelId,
    stats: ChannelId
}



pub const DEFAULT_CONFIG: &str = include_str!("../default_config.toml");

#[cfg(test)]
mod tests {
    use crate::{DEFAULT_CONFIG, Config};

    #[test]
    fn default_config_parses() {
        let _: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
    }
}



fn load_or_create_config() -> anyhow::Result<Config> {
    let config_path = PathBuf::from("config.toml");
    if !config_path.exists() {
        std::fs::write(&config_path, DEFAULT_CONFIG).context("Failed to create default config")?;
    }
    let config_file = String::from_utf8(std::fs::read(config_path).context("failed to read config file")?).context("config file is not utf8!")?;
    Ok(toml::from_str(&config_file).context("Failed to parse config file")?)
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init(); 
    info!("Hello, world!");
    // load config
    let config = load_or_create_config()?;
    
    // validate token
    poise::serenity_prelude::validate_token(&config.main.token).context("Invalid token")?;
    
    


    Ok(())
}
