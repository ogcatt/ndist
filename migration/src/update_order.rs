use sea_orm_migration::{prelude::*, sea_query::extension::postgres::Type};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("Order"))
                    .add_column(ColumnDef::new(Alias::new("refCode")).text().not_null().unique_key())
                    .add_column(ColumnDef::new(Alias::new("refundedAt")).timestamp_with_time_zone())
                    .add_column(ColumnDef::new(Alias::new("preparedAt")).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("Order"))
                    .drop_column(Alias::new("refCode"))
                    .drop_column(Alias::new("refundedAt"))
                    .drop_column(Alias::new("preparedAt"))
                    .to_owned(),
            )
            .await
    }
}
