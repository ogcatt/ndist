use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Modify stock_batches table - convert quantity fields from i32 to f64
        manager
            .alter_table(
                Table::alter()
                    .table(StockBatches::Table)
                    .modify_column(
                        ColumnDef::new(StockBatches::OriginalQuantity)
                            .double()
                            .not_null()
                    )
                    .modify_column(
                        ColumnDef::new(StockBatches::LiveQuantity)
                            .double()
                            .not_null()
                    )
                    .to_owned(),
            )
            .await?;

        // Modify stock_items table - convert default_cost from i32 to f64 and add warning_quantity
        manager
            .alter_table(
                Table::alter()
                    .table(StockItems::Table)
                    .modify_column(
                        ColumnDef::new(StockItems::DefaultCost)
                            .double()
                            .null()
                    )
                    .add_column(
                        ColumnDef::new(StockItems::WarningQuantity)
                            .double()
                            .null()
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert stock_batches table - convert quantity fields back from f64 to i32
        manager
            .alter_table(
                Table::alter()
                    .table(StockBatches::Table)
                    .modify_column(
                        ColumnDef::new(StockBatches::OriginalQuantity)
                            .integer()
                            .not_null()
                    )
                    .modify_column(
                        ColumnDef::new(StockBatches::LiveQuantity)
                            .integer()
                            .not_null()
                    )
                    .to_owned(),
            )
            .await?;

        // Revert stock_items table - convert default_cost back from f64 to i32 and remove warning_quantity
        manager
            .alter_table(
                Table::alter()
                    .table(StockItems::Table)
                    .modify_column(
                        ColumnDef::new(StockItems::DefaultCost)
                            .integer()
                            .null()
                    )
                    .drop_column(StockItems::WarningQuantity)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum StockBatches {
    Table,
    #[iden = "originalQuantity"]
    OriginalQuantity,
    #[iden = "liveQuantity"]
    LiveQuantity,
}

#[derive(Iden)]
enum StockItems {
    Table,
    #[iden = "defaultCost"]
    DefaultCost,
    #[iden = "warningQuantity"]
    WarningQuantity,
}