use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "add_store_settings"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS store_settings (
                    id TEXT PRIMARY KEY,
                    "lockStore" BOOLEAN NOT NULL DEFAULT false,
                    "lockComment" TEXT,
                    "updatedAt" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                INSERT INTO store_settings (id, "lockStore", "lockComment", "updatedAt")
                VALUES ('singleton', false, NULL, NOW())
                ON CONFLICT (id) DO NOTHING;
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS store_settings;")
            .await?;
        Ok(())
    }
}
