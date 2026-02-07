use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add orderItemId to stock_backorder_active_reduce table
        manager
            .alter_table(
                Table::alter()
                    .table(StockBackorderActiveReduce::Table)
                    .add_column(
                        ColumnDef::new(StockBackorderActiveReduce::OrderItemId)
                            .text()
                            .not_null()
                            .default("")
                    )
                    .to_owned(),
            )
            .await?;

        // Add orderItemId to stock_preorder_active_reduce table
        manager
            .alter_table(
                Table::alter()
                    .table(StockPreorderActiveReduce::Table)
                    .add_column(
                        ColumnDef::new(StockPreorderActiveReduce::OrderItemId)
                            .text()
                            .not_null()
                            .default("")
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove orderItemId from stock_backorder_active_reduce table
        manager
            .alter_table(
                Table::alter()
                    .table(StockBackorderActiveReduce::Table)
                    .drop_column(StockBackorderActiveReduce::OrderItemId)
                    .to_owned(),
            )
            .await?;

        // Remove orderItemId from stock_preorder_active_reduce table
        manager
            .alter_table(
                Table::alter()
                    .table(StockPreorderActiveReduce::Table)
                    .drop_column(StockPreorderActiveReduce::OrderItemId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum StockBackorderActiveReduce {
    #[sea_orm(iden = "stock_backorder_active_reduce")]
    Table,
    #[sea_orm(iden = "id")]
    Id,
    #[sea_orm(iden = "orderId")]
    OrderId,
    #[sea_orm(iden = "orderItemId")]
    OrderItemId,
    #[sea_orm(iden = "stockBatchId")]
    StockBatchId,
    #[sea_orm(iden = "stockUnit")]
    StockUnit,
    #[sea_orm(iden = "reductionQuantity")]
    ReductionQuantity,
    #[sea_orm(iden = "active")]
    Active,
    #[sea_orm(iden = "createdAt")]
    CreatedAt,
    #[sea_orm(iden = "updatedAt")]
    UpdatedAt,
}

#[derive(DeriveIden)]
enum StockPreorderActiveReduce {
    #[sea_orm(iden = "stock_preorder_active_reduce")]
    Table,
    #[sea_orm(iden = "id")]
    Id,
    #[sea_orm(iden = "orderId")]
    OrderId,
    #[sea_orm(iden = "orderItemId")]
    OrderItemId,
    #[sea_orm(iden = "stockBatchId")]
    StockBatchId,
    #[sea_orm(iden = "stockUnit")]
    StockUnit,
    #[sea_orm(iden = "reductionQuantity")]
    ReductionQuantity,
    #[sea_orm(iden = "active")]
    Active,
    #[sea_orm(iden = "createdAt")]
    CreatedAt,
    #[sea_orm(iden = "updatedAt")]
    UpdatedAt,
}
