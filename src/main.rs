use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use futures::TryStreamExt;
use tokio::time::sleep;

mod db;
mod scraper;
mod telegram;

use db::DB;
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

    // Wait for 1 minute before starting to scrape
    sleep(Duration::from_secs(60)).await;

    let db = DB::new()?;
    loop {
        // Extract new urls from the web every hour
        let mut urls = scraper::extract_urls().await;
        while let Some(url) = urls.try_next().await? {
            if db.add_url(&url).await? {
                println!("Notifying about {}", url);
                telegram.notify(&db, &url).await?;
            }
        }

        sleep(Duration::from_secs(60 * 60)).await;
    }
}
