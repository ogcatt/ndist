use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add active column to stock_backorder_active_reduce table
        manager
            .alter_table(
                Table::alter()
                    .table(StockBackorderActiveReduce::Table)
                    .add_column(
                        ColumnDef::new(StockBackorderActiveReduce::Active)
                            .boolean()
                            .not_null()
                            .default(true)
                    )
                    .to_owned(),
            )
            .await?;

        // Add active column to stock_preorder_active_reduce table
        manager
            .alter_table(
                Table::alter()
                    .table(StockPreorderActiveReduce::Table)
                    .add_column(
                        ColumnDef::new(StockPreorderActiveReduce::Active)
                            .boolean()
                            .not_null()
                            .default(true)
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove active column from stock_backorder_active_reduce table
        manager
            .alter_table(
                Table::alter()
                    .table(StockBackorderActiveReduce::Table)
                    .drop_column(StockBackorderActiveReduce::Active)
                    .to_owned(),
            )
            .await?;

        // Remove active column from stock_preorder_active_reduce table
        manager
            .alter_table(
                Table::alter()
                    .table(StockPreorderActiveReduce::Table)
                    .drop_column(StockPreorderActiveReduce::Active)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum StockBackorderActiveReduce {
    Table,
    Active,
}

#[derive(DeriveIden)]
enum StockPreorderActiveReduce {
    Table,
    Active,
}
