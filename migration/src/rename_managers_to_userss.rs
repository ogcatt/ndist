use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Rename managers table to users
        manager
            .rename_table(
                Table::rename()
                    .table(Managers::Table, Users::Table)
                    .to_owned()
            )
            .await?;

        // Rename manager_sessions table to user_sessions
        manager
            .rename_table(
                Table::rename()
                    .table(ManagerSessions::Table, UserSessions::Table)
                    .to_owned()
            )
            .await?;

        // Rename manager_id column to user_id in user_sessions table
        manager
            .alter_table(
                Table::alter()
                    .table(UserSessions::Table)
                    .rename_column(ManagerSessions::ManagerId, UserSessions::UserId)
                    .to_owned(),
            )
            .await?;

        // Add admin column to users table and drop permissions column
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(ColumnDef::new(Users::Admin).boolean().not_null().default(false))
                    .drop_column(Users::Permissions)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Reverse the changes
        // First drop admin column and add back permissions
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::Admin)
                    .add_column(ColumnDef::new(Users::Permissions).text().not_null())
                    .to_owned(),
            )
            .await?;

        // Rename user_id back to manager_id
        manager
            .alter_table(
                Table::alter()
                    .table(UserSessions::Table)
                    .rename_column(UserSessions::UserId, ManagerSessions::ManagerId)
                    .to_owned(),
            )
            .await?;

        // Rename tables back
        manager
            .rename_table(
                Table::rename()
                    .table(UserSessions::Table, ManagerSessions::Table)
                    .to_owned()
            )
            .await?;

        manager
            .rename_table(
                Table::rename()
                    .table(Users::Table, Managers::Table)
                    .to_owned()
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Managers {
    Table,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Admin,
    Permissions,
}

#[derive(DeriveIden)]
enum UserSessions {
    Table,
    UserId,
}

#[derive(DeriveIden)]
enum ManagerSessions {
    Table,
    ManagerId,
}
