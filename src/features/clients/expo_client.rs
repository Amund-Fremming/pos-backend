use serde::Serialize;
use serde_json::Value;

const EXPO_PUSH_URL: &str = "https://exp.host/--/api/v2/push/send";

#[derive(Clone)]
pub struct ExpoClient {
    client: reqwest::Client,
}

#[derive(Serialize)]
struct PushMessage<'a> {
    to: &'a str,
    title: &'a str,
    body: &'a str,
}

impl ExpoClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub async fn send(
        &self,
        tokens: &[String],
        title: &str,
        body: &str,
    ) -> Result<Value, reqwest::Error> {
        let messages: Vec<PushMessage> = tokens
            .iter()
            .map(|token| PushMessage {
                to: token,
                title,
                body,
            })
            .collect();

        self.client
            .post(EXPO_PUSH_URL)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&messages)
            .send()
            .await?
            .json::<Value>()
            .await
    }
}
