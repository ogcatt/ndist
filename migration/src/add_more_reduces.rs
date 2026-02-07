use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create stock_backorder_active_reduce table
        manager
            .create_table(
                Table::create()
                    .table(StockBackorderActiveReduce::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StockBackorderActiveReduce::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(StockBackorderActiveReduce::OrderId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockBackorderActiveReduce::StockBatchId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockBackorderActiveReduce::StockUnit)
                            .custom(Alias::new("stock_unit"))
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockBackorderActiveReduce::ReductionQuantity)
                            .double()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockBackorderActiveReduce::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(StockBackorderActiveReduce::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_stock_backorder_active_reduce_order_id")
                            .from(StockBackorderActiveReduce::Table, StockBackorderActiveReduce::OrderId)
                            .to(Order::Table, Order::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_stock_backorder_active_reduce_stock_batch_id")
                            .from(StockBackorderActiveReduce::Table, StockBackorderActiveReduce::StockBatchId)
                            .to(StockBatches::Table, StockBatches::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create stock_preorder_active_reduce table
        manager
            .create_table(
                Table::create()
                    .table(StockPreorderActiveReduce::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StockPreorderActiveReduce::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(StockPreorderActiveReduce::OrderId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockPreorderActiveReduce::StockBatchId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockPreorderActiveReduce::StockUnit)
                            .custom(Alias::new("stock_unit"))
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockPreorderActiveReduce::ReductionQuantity)
                            .double()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockPreorderActiveReduce::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(StockPreorderActiveReduce::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_stock_preorder_active_reduce_order_id")
                            .from(StockPreorderActiveReduce::Table, StockPreorderActiveReduce::OrderId)
                            .to(Order::Table, Order::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_stock_preorder_active_reduce_stock_batch_id")
                            .from(StockPreorderActiveReduce::Table, StockPreorderActiveReduce::StockBatchId)
                            .to(StockBatches::Table, StockBatches::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(StockPreorderActiveReduce::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(StockBackorderActiveReduce::Table).to_owned())
            .await?;

        Ok(())
    }
}

// StockBackorderActiveReduce table identifier
#[derive(DeriveIden)]
enum StockBackorderActiveReduce {
    Table,
    Id,
    #[sea_orm(iden = "orderId")]
    OrderId,
    #[sea_orm(iden = "stockBatchId")]
    StockBatchId,
    #[sea_orm(iden = "stockUnit")]
    StockUnit,
    #[sea_orm(iden = "reductionQuantity")]
    ReductionQuantity,
    #[sea_orm(iden = "createdAt")]
    CreatedAt,
    #[sea_orm(iden = "updatedAt")]
    UpdatedAt,
}

// StockPreorderActiveReduce table identifier
#[derive(DeriveIden)]
enum StockPreorderActiveReduce {
    Table,
    Id,
    #[sea_orm(iden = "orderId")]
    OrderId,
    #[sea_orm(iden = "stockBatchId")]
    StockBatchId,
    #[sea_orm(iden = "stockUnit")]
    StockUnit,
    #[sea_orm(iden = "reductionQuantity")]
    ReductionQuantity,
    #[sea_orm(iden = "createdAt")]
    CreatedAt,
    #[sea_orm(iden = "updatedAt")]
    UpdatedAt,
}

// Referenced table identifiers
#[derive(DeriveIden)]
enum Order {
    #[sea_orm(iden = "Order")]
    Table,
    Id,
}

#[derive(DeriveIden)]
enum StockBatches {
    Table,
    Id,
}
