use sea_orm_migration::{prelude::*, sea_orm::{ConnectionTrait, Statement}};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter()
                .table(Order::Table)
                .add_column_if_not_exists(
                    ColumnDef::new(Order::DiscountId)
                        .string()
                        .null()
                )
                .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter()
                .table(Order::Table)
                .drop_column(Order::DiscountId)
                .to_owned()
        ).await
    }
}

#[derive(DeriveIden)]
pub enum Order {
    #[sea_orm(iden = "Order")]
    Table,
    #[sea_orm(iden = "discountId")]
    DiscountId,
}
