use sea_orm_migration::prelude::*;
use sea_orm_migration::prelude::extension::postgres::Type;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
                    .values([
                        StockBatchLocation::EU,
                    ])
                    .to_owned(),
            )
            .await?;

        // Create the stock_batches table
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
                    .col(
                        ColumnDef::new(StockBatches::Comment)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StockBatches::Supplier)
                            .text()
                            .null(),
                    )
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
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StockBatches::ArrivalDate)
                            .timestamp()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StockBatches::WarehouseLocation)
                            .enumeration(StockBatchLocation::Table, [
                                StockBatchLocation::EU,
                            ])
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockBatches::TrackingUrl)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StockBatches::Status)
                            .enumeration(StockBatchStatus::Table, [
                                StockBatchStatus::Draft,
                                StockBatchStatus::Paid,
                                StockBatchStatus::Complete,
                                StockBatchStatus::Issue,
                            ])
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockBatches::Assembled)
                            .boolean()
                            .not_null()
                            .default(false),
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
                            .name("fk_stock_batches_stock_item_id")
                            .from(StockBatches::Table, StockBatches::StockItemId)
                            .to(StockItems::Table, StockItems::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Remove the assembled column from stock_items
        manager
            .alter_table(
                Table::alter()
                    .table(StockItems::Table)
                    .drop_column(StockItems::Assembled)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add back the assembled column to stock_items
        manager
            .alter_table(
                Table::alter()
                    .table(StockItems::Table)
                    .add_column(
                        ColumnDef::new(StockItems::Assembled)
                            .boolean()
                            .null()
                    )
                    .to_owned(),
            )
            .await?;

        // Drop the stock_batches table
        manager
            .drop_table(Table::drop().table(StockBatches::Table).to_owned())
            .await?;

        // Drop the enums
        manager
            .drop_type(Type::drop().name(StockBatchLocation::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(StockBatchStatus::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum StockItems {
    Table,
    Id,
    Assembled,
}

#[derive(Iden)]
enum StockBatches {
    Table,
    Id,
    #[iden = "stockBatchCode"]
    StockBatchCode,
    #[iden = "stockItemId"]
    StockItemId,
    Comment,
    Supplier,
    #[iden = "originalQuantity"]
    OriginalQuantity,
    #[iden = "liveQuantity"]
    LiveQuantity,
    #[iden = "stockUnitOnCreation"]
    StockUnitOnCreation,
    #[iden = "costUsd"]
    CostUsd,
    #[iden = "arrivalDate"]
    ArrivalDate,
    #[iden = "warehouseLocation"]
    WarehouseLocation,
    #[iden = "trackingUrl"]
    TrackingUrl,
    Status,
    Assembled,
    #[iden = "createdAt"]
    CreatedAt,
    #[iden = "updatedAt"]
    UpdatedAt,
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