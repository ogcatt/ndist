use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .add_column(
                        ColumnDef::new(Payment::ProcessorUrl)
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
                    .table(Payment::Table)
                    .drop_column(Payment::ProcessorUrl)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Payment {
    #[sea_orm(iden = "Payment")]
    Table,
    #[sea_orm(iden = "processorUrl")]
    ProcessorUrl,
}
