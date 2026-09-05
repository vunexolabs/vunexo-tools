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
            // `is_foreign_key_violation()` trusts SQLite's extended result
            // code (`SQLITE_CONSTRAINT_FOREIGNKEY`, 787) — but every
            // `ON DELETE RESTRICT` in this schema fails as an *immediate*
            // constraint check, which SQLite reports under the extended
            // code `SQLITE_CONSTRAINT_TRIGGER` (1811) instead, even though
            // the message text is still "FOREIGN KEY constraint failed".
            // Relying on `is_foreign_key_violation()` alone silently missed
            // every real FK violation this app produces, letting it fall
            // through to the opaque `Database` branch instead of the
            // `Conflict` message callers (customers/products delete) build
            // around it — confirmed by actually deleting a
            // still-referenced row, not by reading sqlx's docs.
            if db_err.is_unique_violation()
                || db_err.is_foreign_key_violation()
                || db_err.message().contains("FOREIGN KEY constraint failed")
            {
                return InfrastructureError::ConstraintViolation(db_err.message().to_string());
            }
        }
        InfrastructureError::Database(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    //! `sqlx::Error` isn't constructible from application code (its
    //! `Database` variant only comes from a real driver), so this is
    //! exercised against a real SQLite connection — not a fake — the same
    //! way `application::customers`/`application::products`'s own
    //! integration tests now do for the end-to-end behavior this enables.
    use super::*;

    #[tokio::test]
    async fn a_restrict_foreign_key_violation_is_classified_as_a_constraint_violation() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("connect");
        sqlx::query("CREATE TABLE parents (id INTEGER PRIMARY KEY)")
            .execute(&pool)
            .await
            .expect("create parents");
        sqlx::query(
            "CREATE TABLE children (id INTEGER PRIMARY KEY, \
             parent_id INTEGER REFERENCES parents(id) ON DELETE RESTRICT)",
        )
        .execute(&pool)
        .await
        .expect("create children");
        sqlx::query("INSERT INTO parents (id) VALUES (1)")
            .execute(&pool)
            .await
            .expect("insert parent");
        sqlx::query("INSERT INTO children (id, parent_id) VALUES (1, 1)")
            .execute(&pool)
            .await
            .expect("insert child");

        let err = sqlx::query("DELETE FROM parents WHERE id = 1")
            .execute(&pool)
            .await
            .expect_err("an in-use parent must not delete");
        let infra: InfrastructureError = err.into();
        assert!(
            matches!(infra, InfrastructureError::ConstraintViolation(_)),
            "got {infra:?}"
        );
    }
}
