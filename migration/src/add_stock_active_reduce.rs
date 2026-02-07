use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the stock_active_reduce table
        manager
            .create_table(
                Table::create()
                    .table(StockActiveReduce::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StockActiveReduce::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(StockActiveReduce::OrderId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockActiveReduce::StockBatchId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockActiveReduce::StockUnit)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockActiveReduce::ReductionQuantity)
                            .double()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockActiveReduce::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(StockActiveReduce::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Add foreign key constraint from stock_active_reduce to Order table
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_stock_active_reduce_order_id")
                    .from(StockActiveReduce::Table, StockActiveReduce::OrderId)
                    .to(Order::Table, Order::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        // Add foreign key constraint from stock_active_reduce to stock_batches table
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_stock_active_reduce_stock_batch_id")
                    .from(StockActiveReduce::Table, StockActiveReduce::StockBatchId)
                    .to(StockBatches::Table, StockBatches::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        // Add indexes for better query performance
        manager
            .create_index(
                Index::create()
                    .name("idx_stock_active_reduce_order_id")
                    .table(StockActiveReduce::Table)
                    .col(StockActiveReduce::OrderId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_stock_active_reduce_stock_batch_id")
                    .table(StockActiveReduce::Table)
                    .col(StockActiveReduce::StockBatchId)
                    .to_owned(),
            )
            .await?;

        // Add composite index for queries filtering by both order and stock batch
        manager
            .create_index(
                Index::create()
                    .name("idx_stock_active_reduce_order_batch")
                    .table(StockActiveReduce::Table)
                    .col(StockActiveReduce::OrderId)
                    .col(StockActiveReduce::StockBatchId)
                    .to_owned(),
            )
            .await?;

        // Add index on created_at for time-based queries
        manager
            .create_index(
                Index::create()
                    .name("idx_stock_active_reduce_created_at")
                    .table(StockActiveReduce::Table)
                    .col(StockActiveReduce::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the entire table (this will automatically drop all foreign keys and indexes)
        manager
            .drop_table(Table::drop().table(StockActiveReduce::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum StockActiveReduce {
    #[sea_orm(iden = "stock_active_reduce")]
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

#[derive(DeriveIden)]
enum Order {
    #[sea_orm(iden = "Order")]
    Table,
    Id,
}

#[derive(DeriveIden)]
enum StockBatches {
    #[sea_orm(iden = "stock_batches")]
    Table,
    Id,
}
