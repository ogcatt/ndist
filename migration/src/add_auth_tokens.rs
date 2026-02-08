use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AuthTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AuthTokens::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AuthTokens::Email).string().not_null())
                    .col(ColumnDef::new(AuthTokens::OtpCode).string().not_null())
                    .col(ColumnDef::new(AuthTokens::Used).boolean().default(false))
                    .col(ColumnDef::new(AuthTokens::Attempts).integer().default(0))
                    .col(ColumnDef::new(AuthTokens::ExpiresAt).timestamp().not_null())
                    .col(
                        ColumnDef::new(AuthTokens::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuthTokens::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum AuthTokens {
    Table,
    Id,
    Email,
    OtpCode,
    Used,
    Attempts,
    ExpiresAt,
    CreatedAt,
}
