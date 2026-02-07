use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // First create the enum type
        manager
            .create_type(
                Type::create()
                    .as_enum(ProductPhaseEnum::Enum)
                    .values([
                        ProductPhaseEnum::Blue,
                        ProductPhaseEnum::Purple,
                        ProductPhaseEnum::Orange,
                    ])
                    .to_owned(),
            )
            .await?;

        // Then add the columns
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .add_column(ColumnDef::new(Products::Priority).integer().null())
                    .add_column(ColumnDef::new(Products::Brand).text().null())
                    .add_column(
                        ColumnDef::new(Products::Phase)
                            .enumeration(
                                ProductPhaseEnum::Enum,
                                [
                                    ProductPhaseEnum::Blue,
                                    ProductPhaseEnum::Purple,
                                    ProductPhaseEnum::Orange,
                                ],
                            )
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(Products::BackOrder)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // First drop the columns
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .drop_column(Products::Priority)
                    .drop_column(Products::Brand)
                    .drop_column(Products::Phase)
                    .drop_column(Products::BackOrder)
                    .to_owned(),
            )
            .await?;

        // Then drop the enum type
        manager
            .drop_type(Type::drop().name(ProductPhaseEnum::Enum).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Products {
    Table,
    Priority,
    Brand,
    Phase,
    #[sea_orm(iden = "backOrder")]
    BackOrder,
}

#[derive(DeriveIden)]
enum ProductPhaseEnum {
    #[sea_orm(iden = "product_phase")]
    Enum,
    #[sea_orm(iden = "BLUE")]
    Blue,
    #[sea_orm(iden = "PURPLE")]
    Purple,
    #[sea_orm(iden = "ORANGE")]
    Orange,
}
