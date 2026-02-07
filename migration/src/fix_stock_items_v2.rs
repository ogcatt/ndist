use sea_orm_migration::prelude::*;
use sea_orm_migration::prelude::extension::postgres::Type;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the StockUnit enum
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

        // Create the StockItems table
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
                    .col(
                        ColumnDef::new(StockItems::Name)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockItems::Description)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StockItems::ThumbnailRef)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StockItems::Unit)
                            .enumeration(StockUnit::Table, [
                                StockUnit::Multiples,
                                StockUnit::Grams,
                                StockUnit::Milliliters,
                            ])
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockItems::AssemblyMinutes)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StockItems::DefaultShippingDays)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StockItems::DefaultCost)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StockItems::IsContainer)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(StockItems::Assembled)
                            .boolean()
                            .null(),
                    )
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
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the table first
        manager
            .drop_table(Table::drop().table(StockItems::Table).to_owned())
            .await?;

        // Then drop the enum
        manager
            .drop_type(Type::drop().name(StockUnit::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum StockItems {
    Table,
    Id,
    #[iden = "pbiSku"]
    PbiSku,
    Name,
    #[iden = "description"]
    Description,
    #[iden = "thumbnailRef"]
    ThumbnailRef,
    Unit,
    #[iden = "assemblyMinutes"]
    AssemblyMinutes,
    #[iden = "defaultShippingDays"]
    DefaultShippingDays,
    #[iden = "defaultCost"]
    DefaultCost,
    #[iden = "isContainer"]
    IsContainer,
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