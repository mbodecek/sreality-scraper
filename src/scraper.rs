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

const PAGE_LOAD_SECONDS: u64 = 10;

pub async fn wait_for_page_load() {
    sleep(Duration::from_secs(PAGE_LOAD_SECONDS)).await;
}

pub async fn extract_urls() -> impl Stream<Item = WebDriverResult<String>> {
    try_stream! {
        // Initialize webdriver with headless chrome
        let mut caps = DesiredCapabilities::chrome();
        caps.insert_base_capability("goog:chromeOptions".to_string(), json!({"args": ["--headless=new", "--window-size=1920,1080"]}));
        let driver = WebDriver::new(env::var("SELENIUM_URL").expect("SELENIUM_URL is not set"), caps).await?;

        // Go to the list of offers
        let hostname = env::var("SREALITY_URL").expect("SREALITY_URL is not set");

        for idx in 0.. {
            // break if no more list urls are set
            let path = match env::var(format!("SREALITY_LIST_{}", idx)) {
                Ok(path) => path,
                Err(_) => if idx == 0 {
                    panic!("SREALITY_LIST_0 is not set");
                } else {
                    break;
                },
            };

            println!("Visiting {}", format!("{}{}", hostname, path));

            // Go to the list of offers
            driver.goto(format!("{}{}", hostname, path)).await?;
            wait_for_page_load().await;

            // Click the button to agree with the ads
            let shadow_root_el = driver
            .find_all(By::XPath(
                "//div[contains(@class,\"szn-cmp-dialog-container\")]",
            ))
            .await?;

            if shadow_root_el.len() > 0 {
                shadow_root_el[0].get_shadow_root()
                .await?
                .find(By::Css("button[data-testid=\"cw-button-agree-with-ads\"]"))
                .await?
                .click()
                .await?;

                wait_for_page_load().await;
            }

            loop {
                let path_els = driver
                    .find_all(By::XPath("//a[starts-with(@href, '/detail/')]"))
                    .await?;

                println!("Found {} offers", path_els.len());

                // find all links to the detail page
                let mut paths = path_els
                    .into_iter()
                    .map(async |link| { let result: String = link.attr("href").await?.unwrap(); Ok::<String, WebDriverError>(result) })
                    .collect::<FuturesUnordered<_>>();

                // yield all the detail page urls
                while let Some(pf) = paths.next().await {
                    let p = pf?;
                    yield format!("{}{}", hostname, p);
                }

                // find and click the button to load more results
                let button = driver
                    .find_all(By::XPath("//button[@data-e2e=\"show-more-btn\"]"))
                    .await?;
                if button.len() > 0 {
                    println!("Clicking next page...");
                    button[0].click().await?;
                    wait_for_page_load().await;
                } else {
                    break;
                }
            }
        }
    }
}
