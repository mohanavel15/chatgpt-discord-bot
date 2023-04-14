use std::env;
use std::process;
use std::result::Result;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use reqwest::header::{HeaderMap, CONTENT_TYPE};
use reqwest::Error;
use serde::{Deserialize, Serialize};

const ENDPOINT: &'static str = "https://api.openai.com/v1/chat/completions";
const ERROR_MESSAGE: &'static str = "Unable to get a response. If this problem continues, please contact the administrator of the bot.";

#[derive(Debug, Deserialize, Serialize)]
struct ChatCompletion {
    id: String,
    object: String,
    created: i64,
    model: String,
    usage: Usage,
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Usage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

#[derive(Debug, Deserialize, Serialize)]
struct Choice {
    message: GPTMessage,
    finish_reason: String,
    index: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct GPTMessage {
    role: String,
    content: String,
}

struct Bot {
    openai_token: String,
    http_client: reqwest::Client,
}

impl Bot {
    async fn ask_gpt(&self, message: GPTMessage) -> Result<GPTMessage, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.openai_token).parse().unwrap(),
        );

        let body = serde_json::json!({
            "model": "gpt-3.5-turbo",
            "messages": vec![message]
        });

        let response = self
            .http_client
            .post(ENDPOINT)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        let output: ChatCompletion = response.json().await?;
        Ok(output.choices[0].message.clone())
    }
}

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let message = GPTMessage {
            role: "user".to_string(),
            content: msg.content.clone(),
        };

        match self.ask_gpt(message).await {
            Ok(response) => {
                msg.reply(&ctx.http, response.content.clone())
                    .await
                    .unwrap();
            }
            Err(_) => {
                msg.reply(&ctx.http, ERROR_MESSAGE).await.unwrap();
            }
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    let discord_token = env::var("DISCORD_TOKEN");
    if discord_token.is_err() {
        println!("Expected a discord token in the environment");
        process::exit(1);
    }

    let openai_token = env::var("OPENAI_TOKEN");
    if openai_token.is_err() {
        println!("Expected a openai token in the environment");
        process::exit(1);
    }

    let discord_token = discord_token.unwrap();
    let openai_token = openai_token.unwrap();
    let http_client = reqwest::Client::new();

    let bot = Bot {
        openai_token,
        http_client,
    };

    let intents = GatewayIntents::DIRECT_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(discord_token, intents)
        .event_handler(bot)
        .await
        .expect("Error creating client");

    if let Err(error) = client.start().await {
        println!("Client error: {}", error.to_string());
        process::exit(1);
    }
}
