//! Business profile use cases. application-architecture.md's module layout.
//! user-flows.md §1 — the setup form is required (gates the whole app) before
//! any other screen is usable; "exactly once" is enforced here.

use std::sync::Arc;

use crate::domain::business::{Business, DEFAULT_CURRENCY_SYMBOL};

use super::error::ApplicationError;
use super::ports::business_repository::BusinessRepository;

pub struct BusinessUseCases {
    repo: Arc<dyn BusinessRepository>,
}

impl BusinessUseCases {
    pub fn new(repo: Arc<dyn BusinessRepository>) -> Self {
        Self { repo }
    }

    /// Precondition: no business row exists yet — enforced here since
    /// "exactly once" is the entire point of this use case.
    pub async fn create_business(
        &self,
        mut business: Business,
    ) -> Result<Business, ApplicationError> {
        if self.repo.get().await?.is_some() {
            return Err(ApplicationError::Validation(
                "a business profile already exists".into(),
            ));
        }
        if business.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "business name is required".into(),
            ));
        }
        // database-schema.md §11's `DEFAULT '₹'` only applies when a column
        // is omitted from the INSERT; since the repository always binds it
        // explicitly, the same default is applied here instead so a blank
        // setup-form field still gets a sensible currency symbol.
        if business.currency_symbol.trim().is_empty() {
            business.currency_symbol = DEFAULT_CURRENCY_SYMBOL.to_string();
        }
        Ok(self.repo.create(business).await?)
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
        Ok(self.repo.update(business).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::infrastructure_error::InfrastructureError;
    use std::sync::Mutex;

    struct FakeBusinessRepository {
        stored: Mutex<Option<Business>>,
    }

    #[async_trait::async_trait]
    impl BusinessRepository for FakeBusinessRepository {
        async fn create(&self, business: Business) -> Result<Business, InfrastructureError> {
            *self.stored.lock().unwrap() = Some(business.clone());
            Ok(business)
        }
        async fn get(&self) -> Result<Option<Business>, InfrastructureError> {
            Ok(self.stored.lock().unwrap().clone())
        }
        async fn update(&self, business: Business) -> Result<Business, InfrastructureError> {
            *self.stored.lock().unwrap() = Some(business.clone());
            Ok(business)
        }
    }

    fn business(name: &str) -> Business {
        Business {
            name: name.to_string(),
            address: None,
            tax_info: None,
            currency_symbol: "₹".to_string(),
        }
    }

    #[tokio::test]
    async fn creating_a_second_business_is_rejected() {
        let uc = BusinessUseCases::new(Arc::new(FakeBusinessRepository {
            stored: Mutex::new(None),
        }));
        uc.create_business(business("Acme"))
            .await
            .expect("first create");
        let err = uc.create_business(business("Other")).await.unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }

    #[tokio::test]
    async fn a_blank_name_is_rejected() {
        let uc = BusinessUseCases::new(Arc::new(FakeBusinessRepository {
            stored: Mutex::new(None),
        }));
        let err = uc.create_business(business("   ")).await.unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));
    }
}
