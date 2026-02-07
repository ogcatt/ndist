use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // First, update any existing NULL values to 'BLUE' with proper type casting
        manager
            .get_connection()
            .execute_unprepared(
                "UPDATE products SET phase = 'BLUE'::product_phase WHERE phase IS NULL",
            )
            .await?;

        // Then alter the column to set default and not null
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE products ALTER COLUMN phase SET DEFAULT 'BLUE'::product_phase, ALTER COLUMN phase SET NOT NULL"
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert the column back to nullable without default
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE products ALTER COLUMN phase DROP DEFAULT, ALTER COLUMN phase DROP NOT NULL"
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Products {
    Table,
    Phase,
}

#[derive(DeriveIden)]
enum ProductPhaseEnum {
    #[sea_orm(iden = "product_phase")]
    Enum,
    #[sea_orm(iden = "BLUE")]
    Blue,
    #[sea_orm(iden = "PURPLE")]
    Purple,
    #[sea_orm(iden = "ORANGE")]
    Orange,
}
