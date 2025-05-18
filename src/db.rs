use std::{env, error::Error};

use futures::lock::Mutex;
use sqlite::{Connection, State};

pub struct DB {
    connection: Mutex<Connection>,
}

pub enum AddUrlOutcome {
    NoChange,
    Added,
    PriceChanged(u64),
}

impl DB {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let connection = sqlite::open(env::var("DB_PATH").expect("DB_PATH is not set"))?;

        // Initialize the DB
        connection.execute("CREATE TABLE IF NOT EXISTS known_urls (url TEXT PRIMARY KEY, price INTEGER NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);")?;
        connection.execute("CREATE TABLE IF NOT EXISTS chats (chat_id INTEGER PRIMARY KEY, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);")?;

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub async fn add_url(&self, url: &str, price: u64) -> Result<AddUrlOutcome, Box<dyn Error>> {
        let conn = self.connection.lock().await;

        // First, check if the URL exists and get the current price
        let mut select_st = conn.prepare("SELECT price FROM known_urls WHERE url = :url;")?;
        select_st.bind((":url", url))?;
        Ok(if select_st.next()? == State::Row {
            let old_price: Option<i64> = select_st.read("price")?;

            match old_price {
                Some(old_price) => {
                    if old_price as u64 == price {
                        return Ok(AddUrlOutcome::NoChange);
                    } else {
                        // Update the price
                        let mut update_st =
                            conn.prepare("UPDATE known_urls SET price = :price WHERE url = :url;")?;
                        update_st.bind((":price", price as i64))?;
                        update_st.bind((":url", url))?;
                        update_st.next()?;
                        AddUrlOutcome::PriceChanged(old_price as u64)
                    }
                }
                None => AddUrlOutcome::NoChange,
            }
        } else {
            // Insert new URL
            let mut insert_st =
                conn.prepare("INSERT INTO known_urls (url, price) VALUES (:url, :price);")?;
            insert_st.bind((":url", url))?;
            insert_st.bind((":price", price as i64))?;
            insert_st.next()?;
            AddUrlOutcome::Added
        })
    }

    pub async fn add_chat_id(&self, chat_id: i64) -> Result<bool, Box<dyn Error>> {
        let query =
            "INSERT INTO chats (chat_id) VALUES (:chat_id) ON CONFLICT(chat_id) DO NOTHING;";
        let conn = self.connection.lock().await;
        let mut st = conn.prepare(query)?;
        st.bind((":chat_id", chat_id))?;
        st.next()?;

        Ok(conn.change_count() > 0)
    }

    pub async fn get_chat_ids(&self) -> Result<Vec<i64>, Box<dyn Error>> {
        let query = "SELECT chat_id FROM chats;";
        let conn = self.connection.lock().await;
        let mut st = conn.prepare(query)?;
        let mut chat_ids = Vec::new();
        while st.next()? == State::Row {
            chat_ids.push(st.read("chat_id")?);
        }

        Ok(chat_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::NamedTempFile;

    async unsafe fn setup_test_db() -> (DB, NamedTempFile) {
        let temp_file = NamedTempFile::new().unwrap();
        env::set_var("DB_PATH", temp_file.path());
        let db = DB::new().unwrap();
        (db, temp_file)
    }

    #[tokio::test]
    async fn test_add_url_new() {
        let (db, _temp_file) = unsafe { setup_test_db().await };

        let result = db.add_url("https://example.com/1", 1000).await.unwrap();
        assert!(matches!(result, AddUrlOutcome::Added));
    }

    #[tokio::test]
    async fn test_add_url_no_change() {
        let (db, _temp_file) = unsafe { setup_test_db().await };

        // First add
        let result = db.add_url("https://example.com/1", 1000).await.unwrap();
        assert!(matches!(result, AddUrlOutcome::Added));

        // Add same URL with same price
        let result = db.add_url("https://example.com/1", 1000).await.unwrap();
        assert!(matches!(result, AddUrlOutcome::NoChange));
    }

    #[tokio::test]
    async fn test_add_url_price_changed() {
        let (db, _temp_file) = unsafe { setup_test_db().await };

        // First add
        let result = db.add_url("https://example.com/1", 1000).await.unwrap();
        assert!(matches!(result, AddUrlOutcome::Added));

        // Add same URL with different price
        let result = db.add_url("https://example.com/1", 2000).await.unwrap();
        match result {
            AddUrlOutcome::PriceChanged(old_price) => assert_eq!(old_price, 1000),
            _ => panic!("Expected PriceChanged outcome"),
        }
    }

    #[tokio::test]
    async fn test_add_url_price_changed_from_null() {
        let (db, _temp_file) = unsafe { setup_test_db().await };

        // First add using SQL directly
        db.connection
            .lock()
            .await
            .execute("INSERT INTO known_urls (url, price) VALUES ('https://example.com/1', NULL);")
            .unwrap();

        // Add same URL with different price
        let result = db.add_url("https://example.com/1", 2000).await.unwrap();

        // Should treat as NoChange
        match result {
            AddUrlOutcome::NoChange => {}
            _ => panic!("Expected NoChange outcome"),
        }
    }
}
