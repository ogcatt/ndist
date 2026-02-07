use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create product_variant_stock_item_relations table
        manager
            .create_table(
                Table::create()
                    .table(ProductVariantStockItemRelations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProductVariantStockItemRelations::ProductVariantId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProductVariantStockItemRelations::StockItemId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProductVariantStockItemRelations::Quantity)
                            .double()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProductVariantStockItemRelations::StockUnitOnCreation)
                            .string()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(ProductVariantStockItemRelations::ProductVariantId)
                            .col(ProductVariantStockItemRelations::StockItemId)
                    )
                    .to_owned(),
            )
            .await?;

        // Create stock_item_relations table
        manager
            .create_table(
                Table::create()
                    .table(StockItemRelations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StockItemRelations::ParentStockItemId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockItemRelations::ChildStockItemId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockItemRelations::Quantity)
                            .double()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockItemRelations::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockItemRelations::UpdatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(StockItemRelations::ParentStockItemId)
                            .col(StockItemRelations::ChildStockItemId)
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ProductVariantStockItemRelations::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(StockItemRelations::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum ProductVariantStockItemRelations {
    Table,
    #[sea_orm(iden = "productVariantId")]
    ProductVariantId,
    #[sea_orm(iden = "stockItemId")]
    StockItemId,
    #[sea_orm(iden = "quantity")]
    Quantity,
    #[sea_orm(iden = "stockUnitOnCreation")]
    StockUnitOnCreation,
}

#[derive(DeriveIden)]
enum StockItemRelations {
    Table,
    #[sea_orm(iden = "parentStockItemId")]
    ParentStockItemId,
    #[sea_orm(iden = "childStockItemId")]
    ChildStockItemId,
    #[sea_orm(iden = "quantity")]
    Quantity,
    #[sea_orm(iden = "createdAt")]
    CreatedAt,
    #[sea_orm(iden = "updatedAt")]
    UpdatedAt,
}