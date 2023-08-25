use std::fmt::Display;

use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub text: Option<String>,
    pub user: User,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub host: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Error {
    pub error_body: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Misskey Error: {}", self.error_body))
    }
}

impl std::error::Error for Error {}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PostParams<T> {
    pub i: String,

    #[serde(flatten)]
    pub body: T,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotesCreateParams {
    pub visibility: String,
    pub visible_user_ids: Vec<String>,
    pub text: Option<String>,
    pub local_only: bool,
    pub reply_id: Option<String>,
}

pub struct MisskeyApi {
    client: Client,
    host: String,
    token: String,
}

impl MisskeyApi {
    pub fn new(host: String, token: String) -> Self {
        let client = Client::builder()
            .user_agent(env!("CARGO_PKG_NAME"))
            .pool_max_idle_per_host(0) // api server close connection in about 90 secs
            .tcp_keepalive(None)
            .build()
            .unwrap();
        MisskeyApi {
            client,
            host,
            token,
        }
    }

    async fn post<T>(&self, endpoint: &str, body: T) -> Result<(), Box<dyn std::error::Error>>
    where
        T: Serialize,
    {
        let r = self
            .client
            .post(format!("https://{}/api/{}", self.host, endpoint))
            .json(&body)
            .send()
            .await?;

        if !r.status().is_success() {
            let error_body = r.text().await?;
            return Err(Box::new(Error { error_body }));
        }

        Ok(())
    }

    pub async fn notes_create(
        &self,
        params: NotesCreateParams,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let with_token = PostParams {
            i: self.token.clone(),
            body: params,
        };
        self.post("notes/create", with_token).await
    }
}
