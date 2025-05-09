use std::{env, error::Error};

use futures::lock::Mutex;
use sqlite::{Connection, State};

pub struct DB {
    connection: Mutex<Connection>,
}

impl DB {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let connection = sqlite::open(env::var("DB_PATH").expect("DB_PATH is not set"))?;

        // Initialize the DB
        connection.execute("CREATE TABLE IF NOT EXISTS known_urls (url TEXT PRIMARY KEY, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);")?;
        connection.execute("CREATE TABLE IF NOT EXISTS chats (chat_id INTEGER PRIMARY KEY, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);")?;

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub async fn add_url(&self, url: &str) -> Result<bool, Box<dyn Error>> {
        let query = "INSERT INTO known_urls (url) VALUES (:url) ON CONFLICT(url) DO NOTHING;";
        let conn = self.connection.lock().await;
        let mut st = conn.prepare(query)?;
        st.bind((":url", url))?;
        st.next()?;

        Ok(conn.change_count() > 0)
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
