//! SQLx-backed `TransactionManager`/`Transaction` implementation.
//! See application/ports/transaction.rs for why this uses `Any` downcasting.

use std::any::Any;

use async_trait::async_trait;
use sqlx::{Sqlite, SqlitePool};

use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::transaction::{Transaction, TransactionManager};

pub struct SqlxTransactionManager {
    pool: SqlitePool,
}

impl SqlxTransactionManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TransactionManager for SqlxTransactionManager {
    async fn begin(&self) -> Result<Box<dyn Transaction>, InfrastructureError> {
        let tx = self
            .pool
            .begin()
            .await
            .map_err(|e| InfrastructureError::Transaction(e.to_string()))?;
        Ok(Box::new(SqlxTx(Some(tx))))
    }
}

pub struct SqlxTx(pub Option<sqlx::Transaction<'static, Sqlite>>);

#[async_trait]
impl Transaction for SqlxTx {
    async fn commit(mut self: Box<Self>) -> Result<(), InfrastructureError> {
        self.0
            .take()
            .expect("commit called twice on the same transaction")
            .commit()
            .await
            .map_err(|e| InfrastructureError::Transaction(e.to_string()))
    }

    async fn rollback(mut self: Box<Self>) -> Result<(), InfrastructureError> {
        self.0
            .take()
            .expect("rollback called twice on the same transaction")
            .rollback()
            .await
            .map_err(|e| InfrastructureError::Transaction(e.to_string()))
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Downcasts the abstract transaction handle back to the concrete SQLx
/// transaction every `sqlite_*_repository` needs to actually run a query.
/// The one place this crate's repositories are allowed to know they're
/// talking to SQLx via a `dyn Transaction` — see application/ports/transaction.rs.
pub fn sqlite_tx(tx: &mut dyn Transaction) -> &mut sqlx::Transaction<'static, Sqlite> {
    tx.as_any_mut()
        .downcast_mut::<SqlxTx>()
        .expect("Transaction must be a SqlxTx — no other TransactionManager implementation exists in V1")
        .0
        .as_mut()
        .expect("transaction already committed or rolled back")
}
