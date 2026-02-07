use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Discounts::Table)
                    .modify_column(
                        ColumnDef::new(Discounts::Code)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Discounts::Table)
                    .modify_column(ColumnDef::new(Discounts::Code).string().not_null())
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum Discounts {
    Table,
    Code,
}
