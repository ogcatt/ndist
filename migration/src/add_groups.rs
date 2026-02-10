use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create groups table
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
                    .col(
                        ColumnDef::new(Groups::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Groups::UpdatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create group_members table
        manager
            .create_table(
                Table::create()
                    .table(GroupMembers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GroupMembers::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(GroupMembers::GroupId).text().not_null())
                    .col(ColumnDef::new(GroupMembers::UserId).text().not_null())
                    .col(
                        ColumnDef::new(GroupMembers::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GroupMembers::UpdatedAt)
                            .date_time()
                            .not_null(),
                    )
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
            .await?;

        // Create unique index to prevent duplicate group memberships
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_group_members_unique")
                    .table(GroupMembers::Table)
                    .col(GroupMembers::GroupId)
                    .col(GroupMembers::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order
        manager
            .drop_table(Table::drop().table(GroupMembers::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Groups::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Groups {
    Table,
    Id,
    Name,
    Description,
    #[sea_orm(iden = "createdAt")]
    CreatedAt,
    #[sea_orm(iden = "updatedAt")]
    UpdatedAt,
}

#[derive(DeriveIden)]
enum GroupMembers {
    Table,
    Id,
    GroupId,
    UserId,
    #[sea_orm(iden = "createdAt")]
    CreatedAt,
    #[sea_orm(iden = "updatedAt")]
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
