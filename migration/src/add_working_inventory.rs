use sea_orm_migration::prelude::*;
use sea_orm_migration::prelude::extension::postgres::Type;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create StockUnit enum
        manager
            .create_type(
                Type::create()
                    .as_enum(StockUnit::Table)
                    .values([
                        StockUnit::Multiples,
                        StockUnit::Grams,
                        StockUnit::Milliliters,
                    ])
                    .to_owned(),
            )
            .await?;

        // Create StockMode enum
        manager
            .create_type(
                Type::create()
                    .as_enum(StockMode::Table)
                    .values([
                        StockMode::Calculated,
                        StockMode::ForceStocked,
                        StockMode::ForceUnstocked,
                    ])
                    .to_owned(),
            )
            .await?;

        // Create StockBatchStatus enum
        manager
            .create_type(
                Type::create()
                    .as_enum(StockBatchStatus::Table)
                    .values([
                        StockBatchStatus::Draft,
                        StockBatchStatus::Paid,
                        StockBatchStatus::Complete,
                        StockBatchStatus::Issue,
                    ])
                    .to_owned(),
            )
            .await?;

        // Create StockBatchLocation enum
        manager
            .create_type(
                Type::create()
                    .as_enum(StockBatchLocation::Table)
                    .values([StockBatchLocation::EU])
                    .to_owned(),
            )
            .await?;

        // Create stock_items table
        manager
            .create_table(
                Table::create()
                    .table(StockItems::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StockItems::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(StockItems::PbiSku)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(StockItems::Name).text().not_null())
                    .col(ColumnDef::new(StockItems::Description).text())
                    .col(ColumnDef::new(StockItems::ThumbnailRef).text())
                    .col(
                        ColumnDef::new(StockItems::Unit)
                            .enumeration(StockUnit::Table, [
                                StockUnit::Multiples,
                                StockUnit::Grams,
                                StockUnit::Milliliters,
                            ])
                            .not_null(),
                    )
                    .col(ColumnDef::new(StockItems::AssemblyMinutes).integer())
                    .col(ColumnDef::new(StockItems::DefaultShippingDays).integer())
                    .col(ColumnDef::new(StockItems::DefaultCost).integer())
                    .col(
                        ColumnDef::new(StockItems::IsContainer)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(StockItems::Assembled).boolean())
                    .col(
                        ColumnDef::new(StockItems::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(StockItems::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create product_stock_item_relations table
        manager
            .create_table(
                Table::create()
                    .table(ProductStockItemRelations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProductStockItemRelations::ProductId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProductStockItemRelations::StockItemId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProductStockItemRelations::Quantity)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProductStockItemRelations::StockUnitOnCreation)
                            .enumeration(StockUnit::Table, [
                                StockUnit::Multiples,
                                StockUnit::Grams,
                                StockUnit::Milliliters,
                            ])
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(ProductStockItemRelations::ProductId)
                            .col(ProductStockItemRelations::StockItemId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_product_stock_relations_product")
                            .from(ProductStockItemRelations::Table, ProductStockItemRelations::ProductId)
                            .to(Products::Table, Products::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_product_stock_relations_stock_item")
                            .from(ProductStockItemRelations::Table, ProductStockItemRelations::StockItemId)
                            .to(StockItems::Table, StockItems::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create stock_batches table
        manager
            .create_table(
                Table::create()
                    .table(StockBatches::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StockBatches::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(StockBatches::StockBatchCode)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(StockBatches::StockItemId)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(StockBatches::Comment).text())
                    .col(ColumnDef::new(StockBatches::Supplier).text())
                    .col(
                        ColumnDef::new(StockBatches::OriginalQuantity)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockBatches::LiveQuantity)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockBatches::StockUnitOnCreation)
                            .enumeration(StockUnit::Table, [
                                StockUnit::Multiples,
                                StockUnit::Grams,
                                StockUnit::Milliliters,
                            ])
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockBatches::CostUsd)
                            .double()
                            .not_null(),
                    )
                    .col(ColumnDef::new(StockBatches::ArrivalDate).timestamp())
                    .col(
                        ColumnDef::new(StockBatches::WarehouseLocation)
                            .enumeration(StockBatchLocation::Table, [StockBatchLocation::EU])
                            .not_null(),
                    )
                    .col(ColumnDef::new(StockBatches::TrackingUrl).text())
                    .col(
                        ColumnDef::new(StockBatches::Status)
                            .enumeration(StockBatchStatus::Table, [
                                StockBatchStatus::Draft,
                                StockBatchStatus::Paid,
                                StockBatchStatus::Complete,
                                StockBatchStatus::Issue,
                            ])
                            .not_null()
                            .default("DRAFT"),
                    )
                    .col(
                        ColumnDef::new(StockBatches::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(StockBatches::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_stock_batches_stock_item")
                            .from(StockBatches::Table, StockBatches::StockItemId)
                            .to(StockItems::Table, StockItems::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order
        manager
            .drop_table(Table::drop().table(StockBatches::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ProductStockItemRelations::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(StockItems::Table).to_owned())
            .await?;

        // Drop enums
        manager
            .drop_type(Type::drop().name(StockBatchLocation::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(StockBatchStatus::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(StockMode::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(StockUnit::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum StockItems {
    Table,
    Id,
    PbiSku,
    Name,
    Description,
    ThumbnailRef,
    Unit,
    AssemblyMinutes,
    DefaultShippingDays,
    DefaultCost,
    IsContainer,
    Assembled,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum ProductStockItemRelations {
    Table,
    ProductId,
    StockItemId,
    Quantity,
    StockUnitOnCreation,
}

#[derive(Iden)]
enum StockBatches {
    Table,
    Id,
    StockBatchCode,
    StockItemId,
    Comment,
    Supplier,
    OriginalQuantity,
    LiveQuantity,
    StockUnitOnCreation,
    CostUsd,
    ArrivalDate,
    WarehouseLocation,
    TrackingUrl,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Products {
    Table,
    Id,
}

#[derive(Iden)]
enum StockUnit {
    Table,
    #[iden = "MULTIPLES"]
    Multiples,
    #[iden = "GRAMS"]
    Grams,
    #[iden = "MILLILITERS"]
    Milliliters,
}

#[derive(Iden)]
enum StockMode {
    Table,
    #[iden = "CALCULATED"]
    Calculated,
    #[iden = "FORCE_STOCKED"]
    ForceStocked,
    #[iden = "FORCE_UNSTOCKED"]
    ForceUnstocked,
}

#[derive(Iden)]
enum StockBatchStatus {
    Table,
    #[iden = "DRAFT"]
    Draft,
    #[iden = "PAID"]
    Paid,
    #[iden = "COMPLETE"]
    Complete,
    #[iden = "ISSUE"]
    Issue,
}

#[derive(Iden)]
enum StockBatchLocation {
    Table,
    #[iden = "EU"]
    EU,
}