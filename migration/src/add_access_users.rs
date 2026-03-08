use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "add_access_users"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .add_column(
                        ColumnDef::new(Products::AccessUsers)
                            .array(ColumnType::Text)
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
                    .table(Products::Table)
                    .drop_column(Products::AccessUsers)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Products {
    Table,
    AccessUsers,
}
