use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CustomerBaskets::Table)
                    .add_column(
                        ColumnDef::new(CustomerBaskets::Locked)
                            .boolean()
                            .not_null()
                            .default(false)
                    )
                    .add_column(
                        ColumnDef::new(CustomerBaskets::PaymentId)
                            .text()
                            .null()
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CustomerBaskets::Table)
                    .drop_column(CustomerBaskets::Locked)
                    .drop_column(CustomerBaskets::PaymentId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum CustomerBaskets {
    Table,
    #[sea_orm(iden = "locked")]
    Locked,
    #[sea_orm(iden = "paymentId")]
    PaymentId,
}
