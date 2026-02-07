use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Update refunded_at column from timestamptz to timestamp
        manager
            .alter_table(
                Table::alter()
                    .table(Order::Table)
                    .modify_column(ColumnDef::new(Order::RefundedAt).timestamp().null())
                    .to_owned(),
            )
            .await?;

        // Update prepared_at column from timestamptz to timestamp
        manager
            .alter_table(
                Table::alter()
                    .table(Order::Table)
                    .modify_column(ColumnDef::new(Order::PreparedAt).timestamp().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert refunded_at column back to timestamptz
        manager
            .alter_table(
                Table::alter()
                    .table(Order::Table)
                    .modify_column(
                        ColumnDef::new(Order::RefundedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Revert prepared_at column back to timestamptz
        manager
            .alter_table(
                Table::alter()
                    .table(Order::Table)
                    .modify_column(
                        ColumnDef::new(Order::PreparedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Order {
    #[sea_orm(iden = "Order")]
    Table,
    #[sea_orm(iden = "refundedAt")]
    RefundedAt,
    #[sea_orm(iden = "preparedAt")]
    PreparedAt,
}
