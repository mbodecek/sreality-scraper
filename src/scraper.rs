use std::env;
use std::time::Duration;

use thirtyfour::CapabilitiesHelper;
use thirtyfour::error::{WebDriverError, WebDriverResult};
use thirtyfour::{By, DesiredCapabilities, WebDriver};

use async_stream::try_stream;
use futures::Stream;
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::time::sleep;

use serde_json::json;

pub async fn extract_urls() -> impl Stream<Item = WebDriverResult<String>> {
    let stream = try_stream! {
        let mut caps = DesiredCapabilities::chrome();
        caps.insert_base_capability("goog:chromeOptions".to_string(), json!({"args": ["--headless"]}));
        let driver = WebDriver::new(env::var("SELENIUM_URL").expect("SELENIUM_URL is not set"), caps).await?;

        let hostname = env::var("SREALITY_URL").expect("SREALITY_URL is not set");

        driver.goto(format!("{}{}", hostname, env::var("SREALITY_LIST").expect("SREALITY_LIST is not set"))).await?;

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
