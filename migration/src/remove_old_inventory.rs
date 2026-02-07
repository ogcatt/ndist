use sea_orm_migration::prelude::*;
use sea_orm_migration::prelude::extension::postgres::Type;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop stock_dependencies table first (due to foreign key constraints)
        manager
            .drop_table(Table::drop().table(StockDependencies::Table).to_owned())
            .await?;

        // Drop stock_items table
        manager
            .drop_table(Table::drop().table(StockItems::Table).to_owned())
            .await?;

        // Drop QuantityType enum
        manager
            .drop_type(
                Type::drop()
                    .if_exists()
                    .name(Alias::new("QuantityType"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Recreate QuantityType enum
        manager
            .create_type(
                Type::create()
                    .as_enum(Alias::new("QuantityType"))
                    .values([
                        Alias::new("GRAMS"),
                        Alias::new("LITERS"),
                        Alias::new("MILLIGRAMS"),
                        Alias::new("MILLILITERS"),
                        Alias::new("UNITS"),
                    ])
                    .to_owned(),
            )
            .await?;

        // Recreate stock_items table
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
                    .col(ColumnDef::new(StockItems::Title).text().not_null())
                    .col(ColumnDef::new(StockItems::VariantId).text())
                    .col(ColumnDef::new(StockItems::Sku).text())
                    .col(ColumnDef::new(StockItems::ImageUrlPath).text())
                    .col(
                        ColumnDef::new(StockItems::IsScoop)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(StockItems::InStockQuantity)
                            .double()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockItems::UnitOfMeasure)
                            .enumeration(
                                Alias::new("QuantityType"),
                                [
                                    Alias::new("GRAMS"),
                                    Alias::new("LITERS"),
                                    Alias::new("MILLIGRAMS"),
                                    Alias::new("MILLILITERS"),
                                    Alias::new("UNITS"),
                                ],
                            )
                            .not_null(),
                    )
                    .col(ColumnDef::new(StockItems::MinStockLevel).double())
                    .col(ColumnDef::new(StockItems::EstimatedCostPerBatch).double())
                    .col(ColumnDef::new(StockItems::BatchQuantity).double())
                    .col(
                        ColumnDef::new(StockItems::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockItems::UpdatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_stock_items_variant_id")
                            .from(StockItems::Table, StockItems::VariantId)
                            .to(Alias::new("product_variants"), Alias::new("id"))
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // Recreate stock_dependencies table
        manager
            .create_table(
                Table::create()
                    .table(StockDependencies::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StockDependencies::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(StockDependencies::ParentStockItemId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockDependencies::ChildStockItemId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockDependencies::RequiredQuantity)
                            .double()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockDependencies::QuantityType)
                            .enumeration(
                                Alias::new("QuantityType"),
                                [
                                    Alias::new("GRAMS"),
                                    Alias::new("LITERS"),
                                    Alias::new("MILLIGRAMS"),
                                    Alias::new("MILLILITERS"),
                                    Alias::new("UNITS"),
                                ],
                            )
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockDependencies::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockDependencies::UpdatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_stock_dependencies_parent_stock_item_id")
                            .from(StockDependencies::Table, StockDependencies::ParentStockItemId)
                            .to(StockItems::Table, StockItems::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_stock_dependencies_child_stock_item_id")
                            .from(StockDependencies::Table, StockDependencies::ChildStockItemId)
                            .to(StockItems::Table, StockItems::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum StockItems {
    Table,
    Id,
    Title,
    VariantId,
    Sku,
    ImageUrlPath,
    IsScoop,
    InStockQuantity,
    UnitOfMeasure,
    MinStockLevel,
    EstimatedCostPerBatch,
    BatchQuantity,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum StockDependencies {
    Table,
    Id,
    ParentStockItemId,
    ChildStockItemId,
    RequiredQuantity,
    QuantityType,
    CreatedAt,
    UpdatedAt,
}