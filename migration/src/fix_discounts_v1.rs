use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Discounts::Table)
                    .drop_column(Discounts::ValidCountries) // Remove the incorrect column
                    .add_column(
                        ColumnDef::new(Discounts::ValidCountries)
                            .array(ColumnType::Text) // Add it back with the correct type
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Discounts::Table)
                    .drop_column(Discounts::ValidCountries) // Remove the corrected column
                    .add_column(
                        ColumnDef::new(Discounts::ValidCountries)
                            .json() // Revert to the original incorrect type
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum Discounts {
    Table,
    #[iden = "validCountries"]
    ValidCountries,
}
