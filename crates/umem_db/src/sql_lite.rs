use anyhow::Result;
use libsql::{Builder, Connection};

pub struct SqlLiteStore {
    pub connection: Connection,
}

impl SqlLiteStore {
    pub async fn new(url: String, auth_token: String) -> Result<Self> {
        Ok(Self {
            connection: Builder::new_remote(url, auth_token)
                .build()
                .await?
                .connect()?,
        })
    }
}
