use std::error::Error;
use std::pin::Pin;
use std::{process::Command, time::Duration};

use std::sync::mpsc::channel;

use async_stream::try_stream;
use futures::stream::{FuturesUnordered, StreamExt};
use futures::{Stream, TryStream, TryStreamExt};
use thirtyfour::error::WebDriverError;
//use fantoccini::{error::WebDriver, wd::Capabilities, ClientBuilder, Locator};
use thirtyfour::{By, DesiredCapabilities, WebDriver, error::WebDriverResult};
use tokio::time::sleep;

/*
    let _ = Command::new("terminal-notifier")
        .args(&[
            "-title",
            "Rust Notification",
            "-message",
            "Your cron job ran!",
        ])
        .output()
        .expect("failed to execute notifier");

*/

/*
async fn extract_urls() -> WebDriverResult<Vec<String>> {
    let caps = DesiredCapabilities::chrome();
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

    let mut urls = Vec::new();
    loop {
        let paths = driver
            .find_all(By::XPath("//a[starts-with(@href, '/detail/prodej/byt/')]"))
            .await?
            .into_iter()
            .map(async |link| link.attr("href").await.unwrap().unwrap())
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;

        urls.extend(paths.iter().map(|p| format!("{}{}", hostname, p)));

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

    // Always explicitly close the browser.
    driver.quit().await?;

    Ok(urls)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let connection = sqlite::open("./db.sqlite").unwrap();
    let query = "CREATE TABLE IF NOT EXISTS known_urls (url TEXT PRIMARY KEY, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);";

    let urls = extract_urls().await?;

    connection.execute(query).unwrap();

    for url in urls {
        let query = "INSERT INTO known_urls (url) VALUES (:url) ON CONFLICT(url) DO NOTHING;";
        let mut st = connection.prepare(query)?;
        st.bind((":url", url.as_str()))?;
        st.next()?;
        if connection.change_count() > 0 {
            println!("{}", url);
        }
    }

    Ok(())
}
*/

async fn extract_urls() -> Pin<Box<dyn Stream<Item = Result<String, Box<dyn Error>>>>> {
    let stream = try_stream! {
        let caps = DesiredCapabilities::chrome();
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
            let paths = driver
                .find_all(By::XPath("//a[starts-with(@href, '/detail/prodej/byt/')]"))
                .await?
                .into_iter()
                .map(async |link| format!("{}{}", hostname, link.attr("href").await.unwrap().unwrap()))
                .collect::<FuturesUnordered<_>>()
                .collect::<Vec<_>>()
                .await;

            for url in paths {
                yield url;
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
    let query = "CREATE TABLE IF NOT EXISTS known_urls (url TEXT PRIMARY KEY, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);";

    let driver = WebDriver::new("http://localhost:61989", DesiredCapabilities::chrome()).await?;
    let mut urls = extract_urls().await;
    // Always explicitly close the browser.
    driver.quit().await?;

    connection.execute(query)?;

    loop {
        let url = urls.try_next().await?;
        if let Some(url) = url {
            let query = "INSERT INTO known_urls (url) VALUES (:url) ON CONFLICT(url) DO NOTHING;";
            let mut st = connection.prepare(query).unwrap();
            st.bind((":url", url.as_str())).unwrap();
            st.next()?;
            if connection.change_count() > 0 {
                println!("{}", url);
            }
        } else {
            break;
        }
    }

    /*urls.try_for_each(|url| async move {
        let query = "INSERT INTO known_urls (url) VALUES (:url) ON CONFLICT(url) DO NOTHING;";
        let mut st = connection.prepare(query).unwrap();
        st.bind((":url", url.as_str())).unwrap();
        st.next()?;
        if connection.change_count() > 0 {
            println!("{}", url);
        }
        Ok(())
    })
    .await?;*/

    /*
    for url in urls {
        let query = "INSERT INTO known_urls (url) VALUES (:url) ON CONFLICT(url) DO NOTHING;";
        let mut st = connection.prepare(query)?;
        st.bind((":url", url.as_str()))?;
        st.next()?;
        if connection.change_count() > 0 {
            println!("{}", url);
        }
    }
    */

    Ok(())
}
