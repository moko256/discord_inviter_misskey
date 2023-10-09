use std::{error::Error, sync::Arc, time::Duration};

use async_tungstenite::{tokio::connect_async, tungstenite::Message};
use futures::{lock::Mutex, Future, SinkExt, StreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::api_misskey::Note;

#[derive(PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "type", content = "body")]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum StreamingMessageSend<Params, Body> {
    Connect(StreamingConnect<Params>),
    Channel(StreamingChannel<Body>),
    Disconnect(StreamingDisconnect),
}

#[derive(PartialEq, Eq, Debug, Deserialize)]
#[serde(tag = "type", content = "body")]
#[serde(rename_all = "lowercase")]
pub enum StreamingMessageRecv<Body> {
    Channel(StreamingChannel<Body>),
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StreamingConnect<Params> {
    channel: String,
    id: String,
    params: Params,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StreamingDisconnect {
    id: String,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StreamingChannel<Body> {
    id: String,
    #[serde(flatten)]
    body_inner: Body,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
#[serde(rename_all = "lowercase")]
pub enum StreamingBodyMain {
    Mention(Note),
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
#[serde(rename_all = "lowercase")]
pub enum StreamingBodyTimeline {
    Note(Note),
}

pub struct MisskeyApiStream {
    host: String,
    token: String,
}

impl MisskeyApiStream {
    pub fn new(host: String, token: String) -> Self {
        MisskeyApiStream { host, token }
    }

    async fn start<P, B, F1, F2>(
        &self,
        send_on_start: &[StreamingMessageSend<P, B>],
        on_ready: impl Fn() -> F1,
        on_message: impl Fn(StreamingMessageRecv<B>) -> F2,
    ) -> Result<(), Box<dyn Error>>
    where
        P: Serialize,
        B: Serialize + DeserializeOwned,
        F1: Future<Output = ()>,
        F2: Future<Output = ()>,
    {
        let url = format!("wss://{}/streaming?i={}", self.host, self.token);
        let (ws, _) = connect_async(url).await?;
        let (mut sink, mut stream) = ws.split();

        for msg in send_on_start {
            let msg = serde_json::to_string(&msg)?;
            sink.send(Message::Text(msg)).await?;
        }

        let ping_sending = Mutex::new(false);

        let sink = Arc::new(Mutex::new(sink));

        let pinging = (|| async {
            loop {
                let result = sink.lock().await.send(Message::Ping(Vec::new())).await;
                match result {
                    Ok(_) => {
                        // Sent ping.
                        *(ping_sending.lock().await) = true;
                    }
                    Err(err) => {
                        log::info!("Failed to send ping.");
                        return Err::<(), Box<dyn Error>>(Box::new(err));
                    }
                }

                // Wait next ping timing.
                tokio::time::sleep(Duration::from_secs(60)).await;

                // Check pong.
                if *(ping_sending.lock().await) {
                    log::info!("Pong unreached.");
                    break;
                }
            }

            Ok::<(), Box<dyn Error>>(())
        })();
        tokio::pin!(pinging);

        on_ready().await;

        loop {
            // Early return
            tokio::select! {
                stream_next = stream.next() => {

                    if let Some(event) = stream_next {
                        let event = event?;

                        if let Message::Ping(d) = &event {
                            sink.lock().await.send(Message::Pong(d.clone())).await?;
                        } else if let Message::Pong(_) = &event {
                            // The `event.into_text()` will return empty string.

                            // Ping from client have reached to server successfully.
                            *(ping_sending.lock().await) = false;
                        } else {
                            let msg = event.into_text()?;

                            if let Ok(msg) = serde_json::from_str(&msg) {
                                on_message(msg).await;
                            } else {
                                // Ignore error to ignore unknown event.
                            }
                        }
                    }

                },
                // Pinging is continuous job.
                result = &mut pinging => {
                    return result
                },
            }
        }
    }

    pub async fn start_main<F1, F2>(
        &self,
        on_ready: impl Fn() -> F1,
        on_event: impl Fn(StreamingBodyMain) -> F2,
    ) -> Result<(), Box<dyn Error>>
    where
        F1: Future<Output = ()>,
        F2: Future<Output = ()>,
    {
        let channel = "main".to_string();
        let id = "0".to_string();
        let connect_msg = StreamingConnect {
            channel,
            id,
            params: (),
        };
        self.start(
            &[StreamingMessageSend::<(), StreamingBodyMain>::Connect(
                connect_msg,
            )],
            on_ready,
            |msg| async {
                let StreamingMessageRecv::Channel(ch) = msg;
                on_event(ch.body_inner).await;
            },
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn start_hybrid_timeline<F1, F2>(
        &self,
        on_ready: impl Fn() -> F1,
        on_event: impl Fn(StreamingBodyTimeline) -> F2,
    ) -> Result<(), Box<dyn Error>>
    where
        F1: Future<Output = ()>,
        F2: Future<Output = ()>,
    {
        let channel = "hybridTimeline".to_string();
        let id = "0".to_string();
        let connect_msg = StreamingConnect {
            channel,
            id,
            params: (),
        };
        self.start(
            &[StreamingMessageSend::<(), StreamingBodyTimeline>::Connect(
                connect_msg,
            )],
            on_ready,
            |msg| async {
                let StreamingMessageRecv::Channel(ch) = msg;
                on_event(ch.body_inner).await;
            },
        )
        .await
    }
}
