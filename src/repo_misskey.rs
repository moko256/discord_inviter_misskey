use std::error::Error;

use chrono::Duration;
use futures::Future;

use crate::{
    api_misskey::{MisskeyApi, Note, NotesCreateParams},
    api_misskey_stream::{MisskeyApiStream, StreamingBodyMain},
    config::Config,
    simple_retry::simple_retry_loop_by_time,
};

pub struct RepoMisskey {
    client: MisskeyApi,
    client_stream: MisskeyApiStream,
}

impl RepoMisskey {
    pub fn new<'a>(config: &'a Config) -> RepoMisskey {
        let client = MisskeyApi::new(
            config.misskey_host.clone(),
            config.misskey_bot_token.clone(),
        );
        let client_stream = MisskeyApiStream::new(
            config.misskey_host.to_string(),
            config.misskey_bot_token.to_string(),
        );

        RepoMisskey {
            client,
            client_stream,
        }
    }

    pub async fn post_reply_dm(
        &self,
        reply_to: &Note,
        message: String,
        local_only: bool,
    ) -> Result<(), Box<dyn Error>> {
        self.client
            .notes_create(NotesCreateParams {
                visibility: "specified".to_string(),
                visible_user_ids: vec![reply_to.user.id.clone()],
                text: Some(message),
                local_only: local_only,
                reply_id: Some(reply_to.id.clone()),
            })
            .await?;

        Ok(())
    }

    pub async fn start_watching_mention<F>(&self, on_mention: impl Fn(Note) -> F)
    where
        F: Future<Output = ()>,
    {
        simple_retry_loop_by_time(Duration::minutes(1), Duration::minutes(30), || async {
            // Start Streaming API connection.
            let result = self
                .client_stream
                .start_main(|msg| async {
                    let StreamingBodyMain::Mention(note) = msg;

                    on_mention(note).await;
                })
                .await;

            if let Err(err) = result {
                log::warn!("Connection closed with error: {}", err);
            } else {
                log::warn!("Connection closed without error.");
            }
        })
        .await;
    }
}
