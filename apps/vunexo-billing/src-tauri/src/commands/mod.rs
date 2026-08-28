//! Thin Tauri command handlers: deserialize input, call into
//! `crate::application`, serialize output. No business logic here.
//! See docs/vunexo-billing/architecture.md and application-architecture.md §5.

use tauri::State;

use crate::application::business::BusinessUseCases;
use crate::application::customers::CustomerUseCases;
use crate::application::ApplicationError;
use crate::domain::business::Business;
use crate::domain::customer::{Customer, CustomerFields, CustomerFilter, CustomerListItem};

/// Round 1 technical spike: proves the React -> Tauri -> Rust round trip.
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {name}! Vunexo Billing is wired up: React -> Tauri -> Rust.")
}

#[tauri::command]
pub async fn create_business(
    business_use_cases: State<'_, BusinessUseCases>,
    business: Business,
) -> Result<Business, ApplicationError> {
    business_use_cases.create_business(business).await
}

#[tauri::command]
pub async fn get_business(
    business_use_cases: State<'_, BusinessUseCases>,
) -> Result<Option<Business>, ApplicationError> {
    business_use_cases.get_business().await
}

#[tauri::command]
pub async fn update_business(
    business_use_cases: State<'_, BusinessUseCases>,
    business: Business,
) -> Result<Business, ApplicationError> {
    business_use_cases.update_business(business).await
}

#[tauri::command]
pub async fn create_customer(
    customer_use_cases: State<'_, CustomerUseCases>,
    fields: CustomerFields,
) -> Result<Customer, ApplicationError> {
    customer_use_cases.create_customer(fields).await
}

#[tauri::command]
pub async fn update_customer(
    customer_use_cases: State<'_, CustomerUseCases>,
    id: i64,
    fields: CustomerFields,
) -> Result<Customer, ApplicationError> {
    customer_use_cases.update_customer(id, fields).await
}

#[tauri::command]
pub async fn archive_customer(
    customer_use_cases: State<'_, CustomerUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    customer_use_cases.archive_customer(id).await
}

#[tauri::command]
pub async fn restore_customer(
    customer_use_cases: State<'_, CustomerUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    customer_use_cases.restore_customer(id).await
}

#[tauri::command]
pub async fn delete_customer(
    customer_use_cases: State<'_, CustomerUseCases>,
    id: i64,
) -> Result<(), ApplicationError> {
    customer_use_cases.delete_customer(id).await
}

#[tauri::command]
pub async fn get_customer(
    customer_use_cases: State<'_, CustomerUseCases>,
    id: i64,
) -> Result<Customer, ApplicationError> {
    customer_use_cases.get_customer(id).await
}

#[tauri::command]
pub async fn list_customers(
    customer_use_cases: State<'_, CustomerUseCases>,
    filter: CustomerFilter,
) -> Result<Vec<CustomerListItem>, ApplicationError> {
    customer_use_cases.list_customers(filter).await
}
