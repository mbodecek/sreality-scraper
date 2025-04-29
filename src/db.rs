use std::error::Error;

use sqlite::Connection;

pub struct DB {
    connection: Connection,
}

impl DB {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let connection = sqlite::open("./db.sqlite")?;

        // Initialize the DB
        connection.execute("CREATE TABLE IF NOT EXISTS known_urls (url TEXT PRIMARY KEY, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);")?;

        Ok(Self { connection })
    }

    pub fn add_url(&self, url: &str) -> Result<bool, Box<dyn Error>> {
        let query = "INSERT INTO known_urls (url) VALUES (:url) ON CONFLICT(url) DO NOTHING;";
        let mut st = self.connection.prepare(query)?;
        st.bind((":url", url))?;
        st.next()?;

        Ok(self.connection.change_count() == 0)
    }
}
