use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ManagerSessions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ManagerSessions::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ManagerSessions::ManagerId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ManagerSessions::Token)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(ManagerSessions::ExpiresAt)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ManagerSessions::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ManagerSessions::UpdatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-manager_sessions-manager_id")
                            .from(ManagerSessions::Table, ManagerSessions::ManagerId)
                            .to(Managers::Table, Managers::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Add indexes for performance
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-manager_sessions-manager_id")
                    .table(ManagerSessions::Table)
                    .col(ManagerSessions::ManagerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-manager_sessions-token")
                    .table(ManagerSessions::Table)
                    .col(ManagerSessions::Token)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-manager_sessions-expires_at")
                    .table(ManagerSessions::Table)
                    .col(ManagerSessions::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ManagerSessions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ManagerSessions {
    Table,
    Id,
    ManagerId,
    Token,
    ExpiresAt,
    #[sea_orm(iden = "createdAt")]
    CreatedAt,
    #[sea_orm(iden = "updatedAt")]
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Managers {
    Table,
    Id,
}