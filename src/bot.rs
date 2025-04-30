use std::collections::HashSet;
use std::env;
use std::error::Error;

use futures::TryStreamExt;
use tokio::sync::RwLock;

use teloxide::Bot;
use teloxide::prelude::Requester;
use teloxide::types::{ChatId, UpdateKind};
use teloxide::update_listeners::AsUpdateStream;

/// Handles listening to new subscriptions and notifying them
pub struct Chats {
    chats: RwLock<HashSet<ChatId>>,
    bot: Bot,
}

impl Chats {
    pub fn new() -> Self {
        Self {
            chats: RwLock::new(HashSet::new()),
            bot: Bot::new(env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set")),
        }
    }

    pub async fn listen(&self) -> Result<(), Box<dyn Error>> {
        let mut polling = teloxide::update_listeners::polling_default(self.bot.clone()).await;

        let mut update_stream = Box::pin(polling.as_stream());
        while let Some(update) = update_stream.try_next().await? {
            if let UpdateKind::Message(message) = update.kind {
                self.chats.write().await.insert(message.chat.id);
                self.bot
                    .send_message(
                        message.chat.id,
                        "Budu ti posílat nové nabídky na byty v Brně!",
                    )
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn notify(&self, url: &str) -> Result<(), Box<dyn Error>> {
        for chat_id in self.chats.read().await.iter() {
            self.bot.send_message(chat_id.clone(), url).await?;
        }

        Ok(())
    }
}
