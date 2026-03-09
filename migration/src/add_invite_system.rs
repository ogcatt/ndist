use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "add_invite_system"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create api_keys table
        manager
            .create_table(
                Table::create()
                    .table(ApiKeys::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ApiKeys::Id).text().not_null().primary_key())
                    .col(ColumnDef::new(ApiKeys::GroupId).text().not_null())
                    .col(ColumnDef::new(ApiKeys::Name).text().not_null())
                    .col(ColumnDef::new(ApiKeys::KeyValue).text().not_null())
                    .col(ColumnDef::new(ApiKeys::IsActive).boolean().not_null().default(true))
                    .col(ColumnDef::new(ApiKeys::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(ApiKeys::LastUsedAt).date_time().null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(ApiKeys::Table, ApiKeys::GroupId)
                            .to(Groups::Table, Groups::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create group_invite_codes table
        manager
            .create_table(
                Table::create()
                    .table(InviteCodes::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(InviteCodes::Id).text().not_null().primary_key())
                    .col(ColumnDef::new(InviteCodes::Code).text().not_null().unique_key())
                    .col(ColumnDef::new(InviteCodes::GroupId).text().not_null())
                    .col(ColumnDef::new(InviteCodes::ApiKeyId).text().null())
                    .col(ColumnDef::new(InviteCodes::UsedByUserId).text().null())
                    .col(ColumnDef::new(InviteCodes::UsedAt).date_time().null())
                    .col(ColumnDef::new(InviteCodes::IsRevoked).boolean().not_null().default(false))
                    .col(ColumnDef::new(InviteCodes::CreatedAt).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(InviteCodes::Table, InviteCodes::GroupId)
                            .to(Groups::Table, Groups::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(InviteCodes::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(ApiKeys::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum ApiKeys {
    Table,
    Id,
    GroupId,
    Name,
    KeyValue,
    IsActive,
    CreatedAt,
    LastUsedAt,
}

#[derive(DeriveIden)]
enum InviteCodes {
    Table,
    Id,
    Code,
    GroupId,
    ApiKeyId,
    UsedByUserId,
    UsedAt,
    IsRevoked,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Groups {
    Table,
    Id,
}
