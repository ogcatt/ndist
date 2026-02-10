use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Groups::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Groups::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Groups::Name).text().not_null())
                    .col(ColumnDef::new(Groups::Description).text().null())
                    .col(ColumnDef::new(Groups::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Groups::UpdatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Groups::UserId).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_groups_user_id")
                            .from(Groups::Table, Groups::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GroupMembers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GroupMembers::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(GroupMembers::GroupId).text().not_null())
                    .col(ColumnDef::new(GroupMembers::UserId).text().not_null())
                    .col(ColumnDef::new(GroupMembers::CreatedAt).timestamp().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_members_group_id")
                            .from(GroupMembers::Table, GroupMembers::GroupId)
                            .to(Groups::Table, Groups::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_members_user_id")
                            .from(GroupMembers::Table, GroupMembers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GroupMembers::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Groups::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Groups {
    Table,
    Id,
    Name,
    Description,
    CreatedAt,
    UpdatedAt,
    UserId,
}

#[derive(DeriveIden)]
enum GroupMembers {
    Table,
    Id,
    GroupId,
    UserId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}