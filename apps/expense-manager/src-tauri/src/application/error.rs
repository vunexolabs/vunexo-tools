//! What a use case can fail with, and what reaches Tauri.
//! application-architecture.md's error-handling section.

use super::ports::infrastructure_error::InfrastructureError;

#[derive(Debug)]
pub enum ApplicationError {
    NotFound { entity: &'static str, id: i64 },
    Validation(String),
    Infrastructure(InfrastructureError),
}

impl std::fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplicationError::NotFound { entity, id } => write!(f, "{entity} {id} not found"),
            ApplicationError::Validation(msg) => write!(f, "{msg}"),
            ApplicationError::Infrastructure(_) => {
                write!(f, "something went wrong, your data is safe")
            }
        }
    }
}

impl std::error::Error for ApplicationError {}

impl From<InfrastructureError> for ApplicationError {
    fn from(err: InfrastructureError) -> Self {
        ApplicationError::Infrastructure(err)
    }
}

/// Serializable shape for the Tauri command boundary — `InfrastructureError`'s
/// own message never crosses this boundary verbatim.
#[derive(serde::Serialize)]
pub struct ApplicationErrorPayload {
    pub kind: &'static str,
    pub message: String,
}

impl From<&ApplicationError> for ApplicationErrorPayload {
    fn from(err: &ApplicationError) -> Self {
        let kind = match err {
            ApplicationError::NotFound { .. } => "not_found",
            ApplicationError::Validation(_) => "validation",
            ApplicationError::Infrastructure(_) => "infrastructure",
        };
        ApplicationErrorPayload {
            kind,
            message: err.to_string(),
        }
    }
}

impl serde::Serialize for ApplicationError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ApplicationErrorPayload::from(self).serialize(serializer)
    }
}
