use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Alter the Payment table to make order_id nullable
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .modify_column(
                        ColumnDef::new(Payment::OrderId)
                            .string()
                            .null(), // Make the column nullable
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert the change by making order_id not nullable
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .modify_column(
                        ColumnDef::new(Payment::OrderId)
                            .string()
                            .not_null(), // Revert to not nullable
                    )
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Payment {
    #[sea_orm(iden = "Payment")]
    Table,
    #[sea_orm(iden = "orderId")]
    OrderId,
}
