use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use futures::TryStreamExt;
use num_format::{Locale, ToFormattedString};
use tokio::time::sleep;

mod db;
mod scraper;
mod telegram;

use db::{AddUrlOutcome, DB};
use telegram::Telegram;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Start listening for new chats
    let telegram = Arc::new(Telegram::new()?);
    tokio::spawn({
        let telegram = Arc::clone(&telegram);
        async move {
            let db = DB::new().unwrap();
            telegram.listen(&db).await.unwrap();
        }
    });

    let db = DB::new()?;
    loop {
        // Extract new offers from the web every hour
        let mut offers = Box::pin(scraper::extract_offers().await);

        loop {
            match offers.try_next().await {
                Ok(Some(offer)) => match db.add_url(&offer.url, offer.price).await? {
                    AddUrlOutcome::Added => {
                        println!("Notifying about {}", offer.url);
                        telegram
                            .notify(
                                &db,
                                &format!(
                                    "Nový byt ({} Kč): {}",
                                    offer.price.to_formatted_string(&Locale::cs),
                                    offer.url
                                ),
                            )
                            .await?;
                    }
                    AddUrlOutcome::PriceChanged(old_price) => {
                        println!(
                            "Price changed from {} to {} for {}",
                            old_price, offer.price, offer.url
                        );
                        telegram
                            .notify(
                                &db,
                                &format!(
                                    "Cena se změnila z {} Kč na {} Kč: {}",
                                    old_price.to_formatted_string(&Locale::cs),
                                    offer.price.to_formatted_string(&Locale::cs),
                                    offer.url
                                ),
                            )
                            .await?;
                    }
                    AddUrlOutcome::NoChange => {}
                },
                Ok(None) => {
                    break;
                }
                Err(e) => {
                    // print to stderr
                    eprintln!("Error: {}", e);
                    break;
                }
            }
        }

        println!("Sleeping for 1 hour");
        sleep(Duration::from_secs(60 * 60)).await;
    }
}
