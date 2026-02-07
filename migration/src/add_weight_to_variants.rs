use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ProductVariants::Table)
                    .add_column(ColumnDef::new(ProductVariants::Weight).double().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ProductVariants::Table)
                    .drop_column(ProductVariants::Weight)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum ProductVariants {
    Table,
    Weight,
}
