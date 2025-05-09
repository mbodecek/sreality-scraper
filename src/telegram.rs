use std::env;
use std::error::Error;

use futures::TryStreamExt;

use teloxide::Bot;
use teloxide::prelude::Requester;
use teloxide::types::{ChatId, UpdateKind};
use teloxide::update_listeners::AsUpdateStream;

use crate::db::DB;

/// Handles listening to new subscriptions and notifying them
pub struct Telegram {
    bot: Bot,
}

impl Telegram {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            bot: Bot::new(env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set")),
        })
    }

    pub async fn listen(&self, db: &DB) -> Result<(), Box<dyn Error>> {
        let mut polling = teloxide::update_listeners::polling_default(self.bot.clone()).await;

        let mut update_stream = Box::pin(polling.as_stream());
        while let Some(update) = update_stream.try_next().await? {
            if let UpdateKind::Message(message) = update.kind {
                db.add_chat_id(message.chat.id.0).await?;
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

    pub async fn notify(&self, db: &DB, url: &str) -> Result<(), Box<dyn Error>> {
        let chat_ids = db.get_chat_ids().await?;
        for chat_id in chat_ids.iter() {
            self.bot.send_message(ChatId(*chat_id), url).await?;
        }

        Ok(())
    }
}
