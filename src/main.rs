use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use futures::TryStreamExt;
use tokio::time::sleep;

mod bot;
mod db;
mod scraper;

use bot::Chats;
use db::DB;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Start listening for new chats
    let chats = Arc::new(Chats::new());
    tokio::spawn({
        let chats = Arc::clone(&chats);
        async move {
            chats.listen().await.unwrap();
        }
    });

    // Wait for 1 minute before starting to scrape
    sleep(Duration::from_secs(60)).await;

    let db = DB::new()?;
    loop {
        // Extract new urls from the web every hour
        let mut urls = scraper::extract_urls().await;
        while let Some(url) = urls.try_next().await? {
            if db.add_url(&url)? {
                println!("Notifying about {}", url);
                chats.notify(&url).await?;
            }
        }

        sleep(Duration::from_secs(60 * 60)).await;
    }
}
