use std::{error::Error, time::Duration};

use async_tungstenite::{tokio::connect_async, tungstenite::Message};
use futures::{Future, SinkExt, StreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::api_misskey::Note;

#[derive(PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "type", content = "body")]
#[serde(rename_all = "lowercase")]
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

    async fn start<'de, P, B, F>(
        &self,
        send_on_start: &[StreamingMessageSend<P, B>],
        on_message: impl Fn(StreamingMessageRecv<B>) -> F,
    ) -> Result<(), Box<dyn Error>>
    where
        P: Serialize,
        B: Serialize + DeserializeOwned,
        F: Future<Output = ()>,
    {
        let url = format!("wss://{}/streaming?i={}", self.host, self.token);
        let (ws, _) = connect_async(url).await?;
        let (mut sink, mut stream) = ws.split();

        for msg in send_on_start {
            let msg = serde_json::to_string(&msg)?;
            sink.feed(Message::Text(msg)).await?;
        }
        sink.flush().await?;

        while let Some(event) = stream.next().await {
            let event = event?;

            if let Message::Ping(d) = &event {
                tokio::time::sleep(Duration::from_secs(1)).await;

                sink.send(Message::Pong(d.clone())).await?;
            } else if let Message::Pong(_) = &event {
                // Do nothing. The `event.into_text()` will return empty string.
            } else {
                let msg = event.into_text()?;

                if let Ok(msg) = serde_json::from_str(&msg) {
                    on_message(msg).await;
                } else {
                    // Ignore error to ignore unknown event.
                }
            }
        }

        Ok(())
    }

    pub async fn start_main<F>(
        &self,
        on_event: impl Fn(StreamingBodyMain) -> F,
    ) -> Result<(), Box<dyn Error>>
    where
        F: Future<Output = ()>,
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
            |msg| async {
                let StreamingMessageRecv::Channel(ch) = msg;
                on_event(ch.body_inner).await;
            },
        )
        .await
    }

    pub async fn start_hybrid_timeline<F>(
        &self,
        on_event: impl Fn(StreamingBodyTimeline) -> F,
    ) -> Result<(), Box<dyn Error>>
    where
        F: Future<Output = ()>,
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
            |msg| async {
                let StreamingMessageRecv::Channel(ch) = msg;
                on_event(ch.body_inner).await;
            },
        )
        .await
    }
}
