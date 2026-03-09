use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "update_categories"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace all existing collection keys with "nootropic"
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"UPDATE products SET collections = ARRAY['nootropic'] WHERE collections IS NOT NULL AND array_length(collections, 1) > 0"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
