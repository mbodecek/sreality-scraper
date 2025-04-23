use std::error::Error;
use std::{process::Command, time::Duration};

use async_stream::try_stream;
use futures::stream::{FuturesUnordered, StreamExt};
use futures::{Stream, TryStreamExt};
use thirtyfour::CapabilitiesHelper;
use thirtyfour::error::{WebDriverError, WebDriverResult};
use thirtyfour::{By, DesiredCapabilities, WebDriver};
use tokio::time::sleep;

use serde_json::json;

async fn extract_urls() -> impl Stream<Item = WebDriverResult<String>> {
    let stream = try_stream! {
        let mut caps = DesiredCapabilities::chrome();
        caps.insert_base_capability("goog:chromeOptions".to_string(), json!({"args": ["--headless"]}));
        let driver = WebDriver::new("http://localhost:61989", caps).await?;

        let hostname = "https://www.sreality.cz";

        driver.goto(format!("{}{}", hostname, "/hledani/prodej/byty/brno?velikost=3%2Bkk%2C4%2Bkk&navic=terasa&vlastnictvi=osobni&cena-od=9256624&cena-do=14032559")).await?;

        sleep(Duration::from_secs(2)).await;

        let shadow_root = driver
            .find(By::XPath(
                "//div[contains(@class,\"szn-cmp-dialog-container\")]",
            ))
            .await?
            .get_shadow_root()
            .await?;

        shadow_root
            .find(By::Css("button[data-testid=\"cw-button-agree-with-ads\"]"))
            .await?
            .click()
            .await?;

        sleep(Duration::from_secs(4)).await;

        loop {
            let mut paths = driver
                .find_all(By::XPath("//a[starts-with(@href, '/detail/prodej/byt/')]"))
                .await?
                .into_iter()
                .map(async |link| { let result: String = link.attr("href").await?.unwrap(); Ok::<String, WebDriverError>(result) })
                .collect::<FuturesUnordered<_>>();

            while let Some(pf) = paths.next().await {
                let p = pf?;
                yield format!("{}{}", hostname, p);
            }

            let button = driver
                .find_all(By::XPath("//button[@data-e2e=\"show-more-btn\"]"))
                .await?;
            if button.len() > 0 {
                button[0].click().await?;
                sleep(Duration::from_secs(4)).await;
            } else {
                break;
            }
        }
    };

    Box::pin(stream)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let connection = sqlite::open("./db.sqlite").unwrap();
    connection.execute("CREATE TABLE IF NOT EXISTS known_urls (url TEXT PRIMARY KEY, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);")?;

    let mut urls = extract_urls().await;
    while let Some(url) = urls.try_next().await? {
        let query = "INSERT INTO known_urls (url) VALUES (:url) ON CONFLICT(url) DO NOTHING;";
        let mut st = connection.prepare(query).unwrap();
        st.bind((":url", url.as_str())).unwrap();
        st.next()?;

        if connection.change_count() > 0 {
            println!("{}", &url);
            let _ = Command::new("terminal-notifier")
                .args(&[
                    "-title",
                    "New Listing Found",
                    "-message",
                    &url,
                    "-open",
                    &url,
                ])
                .output()
                .expect("failed to execute notifier");
        }
    }

    Ok(())
}
