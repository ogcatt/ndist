use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // First, create the enum type
        manager
            .create_type(
                Type::create()
                    .as_enum(ShippingOption::Table)
                    .values([
                        ShippingOption::Tracked,
                        ShippingOption::Express,
                        ShippingOption::TrackedUS,
                    ])
                    .to_owned(),
            )
            .await?;

        // Then add new columns to customer_baskets table
        manager
            .alter_table(
                Table::alter()
                    .table(CustomerBaskets::Table)
                    .add_column(ColumnDef::new(CustomerBaskets::CountryCode).text().null())
                    .add_column(ColumnDef::new(CustomerBaskets::DiscountCode).text().null())
                    .add_column(
                        ColumnDef::new(CustomerBaskets::ShippingOption)
                            .enumeration(
                                ShippingOption::Table,
                                [
                                    ShippingOption::Tracked,
                                    ShippingOption::Express,
                                    ShippingOption::TrackedUS,
                                ],
                            )
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove the new columns from customer_baskets table
        manager
            .alter_table(
                Table::alter()
                    .table(CustomerBaskets::Table)
                    .drop_column(CustomerBaskets::CountryCode)
                    .drop_column(CustomerBaskets::DiscountCode)
                    .drop_column(CustomerBaskets::ShippingOption)
                    .to_owned(),
            )
            .await?;

        // Then drop the enum type
        manager
            .drop_type(Type::drop().name(ShippingOption::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CustomerBaskets {
    Table,
    #[sea_orm(iden = "countryCode")]
    CountryCode,
    #[sea_orm(iden = "discountCode")]
    DiscountCode,
    #[sea_orm(iden = "shippingOption")]
    ShippingOption,
}

#[derive(DeriveIden)]
enum ShippingOption {
    Table,
    #[sea_orm(iden = "TRACKED")]
    Tracked,
    #[sea_orm(iden = "EXPRESS")]
    Express,
    #[sea_orm(iden = "TRACKED_US")]
    TrackedUS,
}
