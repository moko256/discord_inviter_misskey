use std::error::Error;

use config::load_config;
use repo_discord::RepoDiscord;
use repo_misskey::RepoMisskey;

mod api_misskey;
mod api_misskey_stream;
mod config;
mod repo_discord;
mod repo_misskey;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = load_config();

    let (repo_discord, discord_task) = RepoDiscord::create_and_start(&config).await;

    let repo_misskey = RepoMisskey::new(&config);
    let misskey_task = repo_misskey.start_watching_mention(|note| async {
        println!(">>> {:?}", note);
        let note = note;

        if let Some(text) = &note.text {
            if text.starts_with(&config.misskey_bot_username) {
                let result: Result<(), Box<dyn Error>> = (|| async {
                    // Send invite url if the user is local user.
                    if note.user.host == None {
                        // Generate and send invite url.
                        let reason = format!(
                            "@{}@{} ({})",
                            note.user.username, config.misskey_host, note.user.id
                        );
                        let url = repo_discord.generate_invite_url(&reason).await?;

                        let msg = format!(
                            "@{} {}\n{}",
                            note.user.username, config.bot_reply_message_ok_invite, url
                        );

                        repo_misskey.post_reply_dm(&note, msg, true).await?;
                    } else {
                        let msg = format!(
                            "@{} {}",
                            note.user.username, config.bot_reply_message_err_remote_user
                        );
                        repo_misskey.post_reply_dm(&note, msg, false).await?;
                    }

                    Ok(())
                })()
                .await;

                if let Err(err) = result {
                    println!("ERR {}", err);
                }
            }
        }
    });

    let (_, _) = tokio::join!(misskey_task, discord_task);

    Ok(())
}
