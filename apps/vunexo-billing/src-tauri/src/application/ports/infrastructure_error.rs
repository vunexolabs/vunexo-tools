//! What a repository or transaction call can fail with.
//! application-architecture.md §6.

#[derive(Debug)]
pub enum InfrastructureError {
    Database(String),
    ConstraintViolation(String),
    Io(String),
    Transaction(String),
}

impl std::fmt::Display for InfrastructureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InfrastructureError::Database(msg) => write!(f, "database error: {msg}"),
            InfrastructureError::ConstraintViolation(msg) => {
                write!(f, "constraint violation: {msg}")
            }
            InfrastructureError::Io(msg) => write!(f, "I/O error: {msg}"),
            InfrastructureError::Transaction(msg) => write!(f, "transaction error: {msg}"),
        }
    }
}

impl std::error::Error for InfrastructureError {}

impl From<sqlx::Error> for InfrastructureError {
    fn from(err: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db_err) = &err {
            if db_err.is_unique_violation() || db_err.is_foreign_key_violation() {
                return InfrastructureError::ConstraintViolation(db_err.message().to_string());
            }
        }
        InfrastructureError::Database(err.to_string())
    }
}
