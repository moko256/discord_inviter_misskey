use std::fs::read_to_string;

use serde::Deserialize;

#[derive(PartialEq, Eq, Debug, Deserialize)]
pub struct Config {
    pub misskey_host: String,
    pub misskey_bot_username: String,
    pub misskey_bot_token: String,
    pub discord_bot_token: String,
    pub discord_channel_invite: u64,
    pub discord_activity_watching: String,
    pub bot_reply_message_ok_invite: String,
    pub bot_reply_message_err_remote_user: String,
}

pub fn load_config() -> Config {
    let config = read_to_string("bot_config.toml").unwrap();
    parse_config(&config)
}

fn parse_config(config: &str) -> Config {
    toml::from_str(&config).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_config_all() {
        assert_eq!(
            parse_config(&read_to_string("bot_config-template.toml").unwrap()),
            Config {
                misskey_host: "example.com".to_string(),
                misskey_bot_username: "@test".to_string(),
                misskey_bot_token: "misskey-token".to_string(),
                discord_bot_token: "discord-token".to_string(),
                discord_channel_invite: 1234,
                discord_activity_watching: "discord_activity_watching".to_string(),
                bot_reply_message_ok_invite: "bot_reply_message_ok_invite".to_string(),
                bot_reply_message_err_remote_user: "bot_reply_message_err_remote_user".to_string(),
            }
        );
    }

    #[test]
    #[should_panic]
    fn invalid_nothing_all() {
        parse_config("");
    }
}
