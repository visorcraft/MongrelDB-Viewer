use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
    #[error("no database is open")]
    NoDatabase,
    #[error("database error: {0}")]
    Database(String),
    #[error("sql error: {0}")]
    Sql(String),
    #[error("embedding error: {0}")]
    Embedding(String),
    #[error("chat error: {0}")]
    Chat(String),
    #[error("mcp error: {0}")]
    Mcp(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("http error: {0}")]
    Http(String),
}

impl AppError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }

    pub fn db(e: impl ToString) -> Self {
        Self::Database(e.to_string())
    }

    pub fn sql(e: impl ToString) -> Self {
        Self::Sql(e.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
