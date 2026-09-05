use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use crate::application::ports::category_repository::CategoryRepository;
use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::domain::category::{Category, CategoryFields, CategoryId, CategoryListItem};

pub struct SqliteCategoryRepository {
    pool: SqlitePool,
}

impl SqliteCategoryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn category_from_row(row: &sqlx::sqlite::SqliteRow) -> Category {
    Category {
        id: row.get("id"),
        name: row.get("name"),
        default_deductible: row.get("default_deductible"),
    }
}

const SELECT_COLUMNS: &str = "id, name, default_deductible";

#[async_trait]
impl CategoryRepository for SqliteCategoryRepository {
    async fn create(&self, fields: CategoryFields) -> Result<Category, InfrastructureError> {
        let id = sqlx::query("INSERT INTO categories (name, default_deductible) VALUES (?, ?)")
            .bind(&fields.name)
            .bind(fields.default_deductible)
            .execute(&self.pool)
            .await?
            .last_insert_rowid();
        Ok(Category {
            id,
            name: fields.name,
            default_deductible: fields.default_deductible,
        })
    }

    async fn update(
        &self,
        id: CategoryId,
        fields: CategoryFields,
    ) -> Result<Category, InfrastructureError> {
        sqlx::query(
            "UPDATE categories SET name = ?, default_deductible = ?, updated_at = datetime('now') \
             WHERE id = ?",
        )
        .bind(&fields.name)
        .bind(fields.default_deductible)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(Category {
            id,
            name: fields.name,
            default_deductible: fields.default_deductible,
        })
    }

    async fn delete(&self, id: CategoryId) -> Result<(), InfrastructureError> {
        sqlx::query("DELETE FROM categories WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn find_by_id(&self, id: CategoryId) -> Result<Option<Category>, InfrastructureError> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM categories WHERE id = ?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(category_from_row))
    }

    async fn list(&self) -> Result<Vec<CategoryListItem>, InfrastructureError> {
        let sql = format!(
            "SELECT c.{col}, EXISTS (SELECT 1 FROM expenses e WHERE e.category_id = c.id) AS has_expenses \
             FROM categories c ORDER BY c.name",
            col = SELECT_COLUMNS.replace(", ", ", c."),
        );
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|row| CategoryListItem {
                category: category_from_row(row),
                has_expenses: row.get::<bool, _>("has_expenses"),
            })
            .collect())
    }

    async fn has_expenses(&self, id: CategoryId) -> Result<bool, InfrastructureError> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM expenses WHERE category_id = ?)")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        Ok(exists)
    }
}
