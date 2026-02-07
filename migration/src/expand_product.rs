use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .add_column(
                        ColumnDef::new(Products::Subtitle)
                            .text()
                            .null()
                    )
                    .add_column(
                        ColumnDef::new(Products::PreOrder)
                            .boolean()
                            .not_null()
                            .default(false)
                    )
                    .add_column(
                        ColumnDef::new(Products::PreOrderGoal)
                            .double()
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
                    .table(Products::Table)
                    .drop_column(Products::Subtitle)
                    .drop_column(Products::PreOrder)
                    .drop_column(Products::PreOrderGoal)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Products {
    Table,
    Subtitle,
    #[sea_orm(iden = "preOrder")]
    PreOrder,
    #[sea_orm(iden = "preOrderGoal")]
    PreOrderGoal,
}
