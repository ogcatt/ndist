use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add access_groups column (array of text/varchar)
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .add_column(
                        ColumnDef::new(Products::AccessGroups)
                            .array(ColumnType::Text)
                            .null()
                    )
                    .to_owned(),
            )
            .await?;

        // Add show_private_preview column (boolean with default false)
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .add_column(
                        ColumnDef::new(Products::ShowPrivatePreview)
                            .boolean()
                            .not_null()
                            .default(false)
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop show_private_preview column
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .drop_column(Products::ShowPrivatePreview)
                    .to_owned(),
            )
            .await?;

        // Drop access_groups column
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .drop_column(Products::AccessGroups)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Products {
    Table,
    AccessGroups,
    ShowPrivatePreview,
}
