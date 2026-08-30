//! Business use cases. application-architecture.md §4 ("Business" bullet).

use std::path::PathBuf;
use std::sync::Arc;

use crate::domain::business::{is_managed_logo_path, managed_logo_path, Business};

use super::error::ApplicationError;
use super::ports::business_repository::BusinessRepository;
use super::ports::file_writer::FileWriter;
use super::ports::transaction::TransactionManager;

pub struct BusinessUseCases {
    repo: Arc<dyn BusinessRepository>,
    tx_manager: Arc<dyn TransactionManager>,
    file_writer: Arc<dyn FileWriter>,
    data_directory: PathBuf,
}

impl BusinessUseCases {
    pub fn new(
        repo: Arc<dyn BusinessRepository>,
        tx_manager: Arc<dyn TransactionManager>,
        file_writer: Arc<dyn FileWriter>,
        data_directory: PathBuf,
    ) -> Self {
        Self {
            repo,
            tx_manager,
            file_writer,
            data_directory,
        }
    }

    /// Precondition: no business row exists yet (user-flows.md §1) — enforced
    /// here since "exactly once" is the entire point of this use case.
    pub async fn create_business(&self, business: Business) -> Result<Business, ApplicationError> {
        if self.repo.get().await?.is_some() {
            return Err(ApplicationError::Conflict(
                "a business profile already exists".into(),
            ));
        }
        if business.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "business name is required".into(),
            ));
        }
        let business = self.import_logo_if_chosen(business)?;

        let mut tx = self.tx_manager.begin().await?;
        match self.repo.create(&mut *tx, business).await {
            Ok(created) => {
                tx.commit().await?;
                Ok(created)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn get_business(&self) -> Result<Option<Business>, ApplicationError> {
        Ok(self.repo.get().await?)
    }

    pub async fn update_business(&self, business: Business) -> Result<Business, ApplicationError> {
        if business.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "business name is required".into(),
            ));
        }
        let business = self.import_logo_if_chosen(business)?;

        let mut tx = self.tx_manager.begin().await?;
        match self.repo.update(&mut *tx, business).await {
            Ok(updated) => {
                tx.commit().await?;
                Ok(updated)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    /// Copies a newly chosen logo into the app's data directory and rewrites
    /// `logo_path` to the resulting **relative** path, so it is app-managed
    /// from here on: `database-schema.md` §9 requires assets to live there so
    /// a `.vbx` backup can carry them and a restore can put them back.
    ///
    /// The picker (`chooseOpenPath` in Settings) always hands back an
    /// absolute path, so "absolute" is exactly "not imported yet" — an
    /// already-managed path is relative and passes through untouched, which
    /// is also what makes this idempotent: saving the form again without
    /// touching the logo field doesn't re-copy it.
    fn import_logo_if_chosen(&self, mut business: Business) -> Result<Business, ApplicationError> {
        let Some(chosen) = business.logo_path.as_deref() else {
            return Ok(business);
        };
        if is_managed_logo_path(chosen) {
            return Ok(business);
        }

        let source = std::path::Path::new(chosen);
        let extension = source.extension().and_then(|e| e.to_str()).unwrap_or("png");
        let managed = managed_logo_path(extension);
        self.file_writer
            .copy(source, &self.data_directory.join(&managed))?;
        business.logo_path = Some(managed);
        Ok(business)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::infrastructure_error::InfrastructureError;
    use std::sync::Mutex;

    /// Records every `copy` call rather than touching a real filesystem —
    /// this test is about *whether* the use case imports the logo and
    /// rewrites the path, not about `std::fs`, which `StdFileWriter` already
    /// covers.
    struct RecordingFileWriter {
        copies: Mutex<Vec<(PathBuf, PathBuf)>>,
    }

    impl RecordingFileWriter {
        fn new() -> Self {
            Self {
                copies: Mutex::new(Vec::new()),
            }
        }
    }

    impl FileWriter for RecordingFileWriter {
        fn write(&self, _path: &std::path::Path, _bytes: &[u8]) -> Result<(), InfrastructureError> {
            Ok(())
        }

        fn copy(
            &self,
            source: &std::path::Path,
            destination: &std::path::Path,
        ) -> Result<(), InfrastructureError> {
            self.copies
                .lock()
                .unwrap()
                .push((source.to_path_buf(), destination.to_path_buf()));
            Ok(())
        }
    }

    fn business(logo_path: Option<&str>) -> Business {
        Business {
            name: "Acme".to_string(),
            logo_path: logo_path.map(str::to_string),
            address: None,
            phone: None,
            email: None,
            gstin: None,
            bank_details: None,
            upi_id: None,
        }
    }

    #[test]
    fn a_freshly_chosen_absolute_logo_is_copied_and_the_path_rewritten() {
        let writer = Arc::new(RecordingFileWriter::new());
        let uc = BusinessUseCases {
            repo: unimplemented_repo(),
            tx_manager: unimplemented_tx_manager(),
            file_writer: writer.clone(),
            data_directory: PathBuf::from("/data"),
        };
        let result = uc
            .import_logo_if_chosen(business(Some("/Users/me/Documents/logo.png")))
            .unwrap();

        assert_eq!(
            result.logo_path.as_deref(),
            Some("assets/business-logo.png")
        );
        let copies = writer.copies.lock().unwrap();
        assert_eq!(copies.len(), 1);
        assert_eq!(copies[0].0, PathBuf::from("/Users/me/Documents/logo.png"));
        assert_eq!(copies[0].1, PathBuf::from("/data/assets/business-logo.png"));
    }

    #[test]
    fn an_already_managed_logo_is_left_alone_and_not_recopied() {
        let writer = Arc::new(RecordingFileWriter::new());
        let uc = BusinessUseCases {
            repo: unimplemented_repo(),
            tx_manager: unimplemented_tx_manager(),
            file_writer: writer.clone(),
            data_directory: PathBuf::from("/data"),
        };
        let result = uc
            .import_logo_if_chosen(business(Some("assets/business-logo.png")))
            .unwrap();

        assert_eq!(
            result.logo_path.as_deref(),
            Some("assets/business-logo.png")
        );
        assert!(writer.copies.lock().unwrap().is_empty());
    }

    #[test]
    fn no_logo_chosen_is_a_no_op() {
        let writer = Arc::new(RecordingFileWriter::new());
        let uc = BusinessUseCases {
            repo: unimplemented_repo(),
            tx_manager: unimplemented_tx_manager(),
            file_writer: writer.clone(),
            data_directory: PathBuf::from("/data"),
        };
        let result = uc.import_logo_if_chosen(business(None)).unwrap();
        assert_eq!(result.logo_path, None);
        assert!(writer.copies.lock().unwrap().is_empty());
    }

    fn unimplemented_repo() -> Arc<dyn BusinessRepository> {
        struct Unimplemented;
        #[async_trait::async_trait]
        impl BusinessRepository for Unimplemented {
            async fn create(
                &self,
                _tx: &mut dyn crate::application::ports::transaction::Transaction,
                _business: Business,
            ) -> Result<Business, InfrastructureError> {
                unimplemented!("not exercised by these tests")
            }
            async fn get(&self) -> Result<Option<Business>, InfrastructureError> {
                unimplemented!("not exercised by these tests")
            }
            async fn update(
                &self,
                _tx: &mut dyn crate::application::ports::transaction::Transaction,
                _business: Business,
            ) -> Result<Business, InfrastructureError> {
                unimplemented!("not exercised by these tests")
            }
        }
        Arc::new(Unimplemented)
    }

    fn unimplemented_tx_manager() -> Arc<dyn TransactionManager> {
        struct Unimplemented;
        #[async_trait::async_trait]
        impl TransactionManager for Unimplemented {
            async fn begin(
                &self,
            ) -> Result<
                Box<dyn crate::application::ports::transaction::Transaction>,
                InfrastructureError,
            > {
                unimplemented!("not exercised by these tests")
            }
        }
        Arc::new(Unimplemented)
    }
}
