use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add add_to_email_list column
        manager
            .alter_table(
                Table::alter()
                    .table(Order::Table)
                    .add_column(
                        ColumnDef::new(Order::AddToEmailList)
                            .boolean()
                            .not_null()
                            .default(true)
                    )
                    .to_owned(),
            )
            .await?;

        // Add tracking_url column
        manager
            .alter_table(
                Table::alter()
                    .table(Order::Table)
                    .add_column(
                        ColumnDef::new(Order::TrackingUrl)
                            .text()
                            .null()
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove tracking_url column
        manager
            .alter_table(
                Table::alter()
                    .table(Order::Table)
                    .drop_column(Order::TrackingUrl)
                    .to_owned(),
            )
            .await?;

        // Remove add_to_email_list column
        manager
            .alter_table(
                Table::alter()
                    .table(Order::Table)
                    .drop_column(Order::AddToEmailList)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Order {
    #[sea_orm(iden = "Order")]
    Table,
    #[sea_orm(iden = "addToEmailList")]
    AddToEmailList,
    #[sea_orm(iden = "trackingUrl")]
    TrackingUrl,
}
