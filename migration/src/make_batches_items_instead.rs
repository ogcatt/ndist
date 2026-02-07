use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // First, drop the foreign key constraints
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_stock_backorder_active_reduce_stock_batch_id")
                    .table(StockBackorderActiveReduce::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_stock_preorder_active_reduce_stock_batch_id")
                    .table(StockPreorderActiveReduce::Table)
                    .to_owned(),
            )
            .await?;

        // Drop the stockBatchId columns
        manager
            .alter_table(
                Table::alter()
                    .table(StockBackorderActiveReduce::Table)
                    .drop_column(StockBackorderActiveReduce::StockBatchId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(StockPreorderActiveReduce::Table)
                    .drop_column(StockPreorderActiveReduce::StockBatchId)
                    .to_owned(),
            )
            .await?;

        // Add the new stockItemId columns with default value
        manager
            .alter_table(
                Table::alter()
                    .table(StockBackorderActiveReduce::Table)
                    .add_column(
                        ColumnDef::new(StockBackorderActiveReduce::StockItemId)
                            .text()
                            .not_null()
                            .default("3916c81c-d0aa-401c-90a6-a76911fc81c0")
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(StockPreorderActiveReduce::Table)
                    .add_column(
                        ColumnDef::new(StockPreorderActiveReduce::StockItemId)
                            .text()
                            .not_null()
                            .default("3916c81c-d0aa-401c-90a6-a76911fc81c0")
                    )
                    .to_owned(),
            )
            .await?;

        // Add new foreign key constraints pointing to stock_items table
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_stock_backorder_active_reduce_stock_item_id")
                    .from(StockBackorderActiveReduce::Table, StockBackorderActiveReduce::StockItemId)
                    .to(StockItems::Table, StockItems::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_stock_preorder_active_reduce_stock_item_id")
                    .from(StockPreorderActiveReduce::Table, StockPreorderActiveReduce::StockItemId)
                    .to(StockItems::Table, StockItems::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the new foreign key constraints
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_stock_backorder_active_reduce_stock_item_id")
                    .table(StockBackorderActiveReduce::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_stock_preorder_active_reduce_stock_item_id")
                    .table(StockPreorderActiveReduce::Table)
                    .to_owned(),
            )
            .await?;

        // Drop the stockItemId columns
        manager
            .alter_table(
                Table::alter()
                    .table(StockBackorderActiveReduce::Table)
                    .drop_column(StockBackorderActiveReduce::StockItemId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(StockPreorderActiveReduce::Table)
                    .drop_column(StockPreorderActiveReduce::StockItemId)
                    .to_owned(),
            )
            .await?;

        // Add back the stockBatchId columns
        manager
            .alter_table(
                Table::alter()
                    .table(StockBackorderActiveReduce::Table)
                    .add_column(
                        ColumnDef::new(StockBackorderActiveReduce::StockBatchId)
                            .text()
                            .not_null()
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(StockPreorderActiveReduce::Table)
                    .add_column(
                        ColumnDef::new(StockPreorderActiveReduce::StockBatchId)
                            .text()
                            .not_null()
                    )
                    .to_owned(),
            )
            .await?;

        // Recreate the original foreign key constraints pointing back to stock_batches
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_stock_backorder_active_reduce_stock_batch_id")
                    .from(StockBackorderActiveReduce::Table, StockBackorderActiveReduce::StockBatchId)
                    .to(StockBatches::Table, StockBatches::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_stock_preorder_active_reduce_stock_batch_id")
                    .from(StockPreorderActiveReduce::Table, StockPreorderActiveReduce::StockBatchId)
                    .to(StockBatches::Table, StockBatches::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum StockBackorderActiveReduce {
    Table,
    #[sea_orm(iden = "stockBatchId")]
    StockBatchId,
    #[sea_orm(iden = "stockItemId")]
    StockItemId,
}

#[derive(DeriveIden)]
enum StockPreorderActiveReduce {
    Table,
    #[sea_orm(iden = "stockBatchId")]
    StockBatchId,
    #[sea_orm(iden = "stockItemId")]
    StockItemId,
}

#[derive(DeriveIden)]
enum StockItems {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum StockBatches {
    Table,
    Id,
}
