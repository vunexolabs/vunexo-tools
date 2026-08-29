//! Settings use cases. application-architecture.md §4 ("Settings" bullet).

use std::sync::Arc;

use crate::domain::settings::{Settings, SettingsFields};

use super::error::ApplicationError;
use super::ports::invoice_repository::InvoiceRepository;
use super::ports::settings_repository::SettingsRepository;
use super::ports::transaction::TransactionManager;

pub struct SettingsUseCases {
    repo: Arc<dyn SettingsRepository>,
    invoice_repo: Arc<dyn InvoiceRepository>,
    tx_manager: Arc<dyn TransactionManager>,
}

impl SettingsUseCases {
    pub fn new(
        repo: Arc<dyn SettingsRepository>,
        invoice_repo: Arc<dyn InvoiceRepository>,
        tx_manager: Arc<dyn TransactionManager>,
    ) -> Self {
        Self {
            repo,
            invoice_repo,
            tx_manager,
        }
    }

    pub async fn get_settings(&self) -> Result<Settings, ApplicationError> {
        Ok(self.repo.get().await?)
    }

    /// `invoice_number_format` becomes read-only once any invoice has been
    /// issued (database-schema.md §7) — enforced here, not left to the UI
    /// to remember.
    pub async fn update_settings(
        &self,
        fields: SettingsFields,
    ) -> Result<Settings, ApplicationError> {
        let current = self.repo.get().await?;
        if fields.invoice_number_format != current.invoice_number_format
            && self.invoice_repo.has_any_issued().await?
        {
            return Err(ApplicationError::Conflict(
                "invoice numbering format can't be changed after the first invoice has been issued"
                    .into(),
            ));
        }

        let mut tx = self.tx_manager.begin().await?;
        match self.repo.update(&mut *tx, fields).await {
            Ok(settings) => {
                tx.commit().await?;
                Ok(settings)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }
}
