use std::{error::Error, sync::Arc};

use serde_json::Number;
use serenity::{
    async_trait,
    http::Http,
    json::JsonMap,
    model::prelude::{Activity, Ready},
    prelude::{Context, EventHandler},
    Client,
};
use tokio::task::JoinHandle;

use crate::config::Config;

pub struct RepoDiscord {
    http: Arc<Http>,
    ch_invite: u64,
}

impl RepoDiscord {
    pub async fn create_and_start(config: &Config) -> (RepoDiscord, JoinHandle<()>) {
        let watching = config.discord_activity_watching.to_string();

        let mut client = Client::builder(config.discord_bot_token.to_string(), Default::default())
            .event_handler(Handler { watching })
            .await
            .unwrap();

        let http = Arc::clone(&client.cache_and_http.http);

        let handle = tokio::spawn(async move {
            client.start().await.unwrap();
        });

        let ch_invite = config.discord_channel_invite;

        (RepoDiscord { http, ch_invite }, handle)
    }

    pub async fn generate_invite_url(&self, reason: &str) -> Result<String, Box<dyn Error>> {
        let mut map = JsonMap::with_capacity(12);

        map.insert(
            "max_age".to_string(),
            serde_json::Value::Number(Number::from(3600)), // 1 hour
        );

        map.insert(
            "max_uses".to_string(),
            serde_json::Value::Number(Number::from(1)),
        );

        map.insert("unique".to_string(), serde_json::Value::Bool(true));

        let invite = self
            .http
            .create_invite(self.ch_invite, &map, Some(reason))
            .await?;

        Ok(format!("https://discord.gg/{}", invite.code))
    }
}

struct Handler {
    watching: String,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _data_about_bot: Ready) {
        ctx.set_activity(Activity::watching(self.watching.to_string()))
            .await;
    }
}
