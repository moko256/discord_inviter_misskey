use std::error::Error;

use chrono::{DateTime, Duration, Utc};
use futures::Future;
use tokio::time::sleep;

use crate::{
    api_misskey::{MisskeyApi, Note, NotesCreateParams},
    api_misskey_stream::{MisskeyApiStream, StreamingBodyMain},
    config::Config,
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
        let mut wait_next = 0;
        let wait_max = 10;
        let minimum_running_duration = Duration::minutes(1);

        loop {
            let minimum_running = Utc::now() + minimum_running_duration;
            let result = self
                .client_stream
                .start_main(|msg| async {
                    let StreamingBodyMain::Mention(note) = msg;

                    on_mention(note).await;
                })
                .await;

            if let Err(err) = result {
                println!("ERR {}", err);
            }

            let now = Utc::now();

            let wait_until;
            if wait_next > 0 && (now > minimum_running) {
                // Last retry was successful but error occur again.
                wait_until = Utc::now();

                wait_next = 0;
            } else {
                // Error looping
                wait_until = now + Duration::minutes(wait_next);

                wait_next = wait_max.min(1.max(wait_next) * 2);
            }

            Self::wait_until(wait_until).await;

            println!("next {}", wait_next);
        }
    }

    async fn wait_until(when: DateTime<Utc>) {
        let wait_until = when - Utc::now();

        match wait_until.to_std() {
            Ok(wait_until) => {
                sleep(wait_until).await;
            }
            Err(_err) => {}
        }
    }
}
