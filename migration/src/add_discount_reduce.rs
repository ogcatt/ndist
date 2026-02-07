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
                    .add_column(
                        ColumnDef::new(Discounts::ActiveReduceQuantity)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Discounts::Table)
                    .drop_column(Discounts::ActiveReduceQuantity)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Discounts {
    #[sea_orm(iden = "discounts")]
    Table,
    #[sea_orm(iden = "activeReduceQuantity")]
    ActiveReduceQuantity,
}
