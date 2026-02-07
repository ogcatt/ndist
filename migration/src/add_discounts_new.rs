use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create DiscountType enum
        manager
            .create_type(
                Type::create()
                    .as_enum(DiscountType::Table)
                    .values([
                        DiscountType::Percentage,
                        DiscountType::FixedAmount,
                        DiscountType::PercentageOnShipping,
                        DiscountType::FixedAmountOnShipping,
                    ])
                    .to_owned(),
            )
            .await?;

        // Create affiliate_users table
        manager
            .create_table(
                Table::create()
                    .table(AffiliateUsers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AffiliateUsers::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AffiliateUsers::Level).integer().not_null())
                    .col(ColumnDef::new(AffiliateUsers::Email).text().not_null())
                    .col(ColumnDef::new(AffiliateUsers::Country).text().not_null())
                    .col(
                        ColumnDef::new(AffiliateUsers::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(AffiliateUsers::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(AffiliateUsers::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create discounts table
        manager
            .create_table(
                Table::create()
                    .table(Discounts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Discounts::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Discounts::Code)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Discounts::AffiliateId).text())
                    .col(
                        ColumnDef::new(Discounts::Active)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Discounts::DiscountType)
                            .enumeration(
                                DiscountType::Table,
                                [
                                    DiscountType::Percentage,
                                    DiscountType::FixedAmount,
                                    DiscountType::PercentageOnShipping,
                                    DiscountType::FixedAmountOnShipping,
                                ],
                            )
                            .not_null(),
                    )
                    .col(ColumnDef::new(Discounts::DiscountPercentage).double())
                    .col(ColumnDef::new(Discounts::DiscountAmount).double())
                    .col(ColumnDef::new(Discounts::AmountUsed).double())
                    .col(ColumnDef::new(Discounts::MaximumUses).integer())
                    .col(
                        ColumnDef::new(Discounts::DiscountUsed)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Discounts::ValidCountries).json())
                    .col(ColumnDef::new(Discounts::ValidAfterXProducts).integer())
                    .col(ColumnDef::new(Discounts::ValidAfterXTotal).double())
                    .col(
                        ColumnDef::new(Discounts::AutoApply)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Discounts::ExpireAt).timestamp().not_null())
                    .col(
                        ColumnDef::new(Discounts::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Discounts::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-discounts-affiliate_id")
                            .from(Discounts::Table, Discounts::AffiliateId)
                            .to(AffiliateUsers::Table, AffiliateUsers::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create affiliate_withdrawls table
        manager
            .create_table(
                Table::create()
                    .table(AffiliateWithdrawls::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AffiliateWithdrawls::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AffiliateWithdrawls::AffiliateId)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AffiliateWithdrawls::Crypto)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AffiliateWithdrawls::CryptoAddress)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AffiliateWithdrawls::TxId).text())
                    .col(
                        ColumnDef::new(AffiliateWithdrawls::Completed)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(AffiliateWithdrawls::Cancelled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(AffiliateWithdrawls::CompletedAt).timestamp())
                    .col(
                        ColumnDef::new(AffiliateWithdrawls::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(AffiliateWithdrawls::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-affiliate_withdrawls-affiliate_id")
                            .from(AffiliateWithdrawls::Table, AffiliateWithdrawls::AffiliateId)
                            .to(AffiliateUsers::Table, AffiliateUsers::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for better query performance
        manager
            .create_index(
                Index::create()
                    .name("idx-discounts-code")
                    .table(Discounts::Table)
                    .col(Discounts::Code)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx-discounts-affiliate_id")
                    .table(Discounts::Table)
                    .col(Discounts::AffiliateId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx-discounts-active")
                    .table(Discounts::Table)
                    .col(Discounts::Active)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx-affiliate_users-email")
                    .table(AffiliateUsers::Table)
                    .col(AffiliateUsers::Email)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order due to foreign key constraints
        manager
            .drop_table(Table::drop().table(AffiliateWithdrawls::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Discounts::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(AffiliateUsers::Table).to_owned())
            .await?;

        // Drop the enum type
        manager
            .drop_type(Type::drop().name(DiscountType::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum DiscountType {
    Table,
    #[iden = "PERCENTAGE"]
    Percentage,
    #[iden = "FIXED_AMOUNT"]
    FixedAmount,
    #[iden = "PERCENTAGE_ON_SHIPPING"]
    PercentageOnShipping,
    #[iden = "FIXED_AMOUNT_ON_SHIPPING"]
    FixedAmountOnShipping,
}

#[derive(Iden)]
enum AffiliateUsers {
    Table,
    Id,
    Level,
    Email,
    Country,
    Enabled,
    #[iden = "createdAt"]
    CreatedAt,
    #[iden = "updatedAt"]
    UpdatedAt,
}

#[derive(Iden)]
enum Discounts {
    Table,
    Id,
    Code,
    #[iden = "affiliateId"]
    AffiliateId,
    Active,
    #[iden = "discountType"]
    DiscountType,
    #[iden = "discountPercentage"]
    DiscountPercentage,
    #[iden = "discountAmount"]
    DiscountAmount,
    #[iden = "amountUsed"]
    AmountUsed,
    #[iden = "maximumUses"]
    MaximumUses,
    #[iden = "discountUsed"]
    DiscountUsed,
    #[iden = "validCountries"]
    ValidCountries,
    #[iden = "validAfterXProducts"]
    ValidAfterXProducts,
    #[iden = "validAfterXTotal"]
    ValidAfterXTotal,
    #[iden = "autoApply"]
    AutoApply,
    #[iden = "expireAt"]
    ExpireAt,
    #[iden = "createdAt"]
    CreatedAt,
    #[iden = "updatedAt"]
    UpdatedAt,
}

#[derive(Iden)]
enum AffiliateWithdrawls {
    Table,
    Id,
    #[iden = "affiliateId"]
    AffiliateId,
    Crypto,
    #[iden = "cryptoAddress"]
    CryptoAddress,
    #[iden = "txId"]
    TxId,
    Completed,
    Cancelled,
    #[iden = "completedAt"]
    CompletedAt,
    #[iden = "createdAt"]
    CreatedAt,
    #[iden = "updatedAt"]
    UpdatedAt,
}
