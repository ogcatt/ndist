use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .drop_column(Products::Collections) // Remove the incorrect column
                    .add_column(
                        ColumnDef::new(Products::Collections)
                            .array(ColumnType::Text) // Add it back with the correct type
                            .null()
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .drop_column(Products::Collections) // Remove the corrected column
                    .add_column(
                        ColumnDef::new(Products::Collections)
                            .json() // Revert to the original incorrect type
                            .null()
                    )
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum Products {
    Table,
    Collections,
}