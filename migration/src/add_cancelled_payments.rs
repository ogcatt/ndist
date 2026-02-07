use sea_orm_migration::prelude::*;

use sea_orm_migration::prelude::extension::postgres::Type;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add the new 'CANCELLED' value to the PaymentStatus enum
        manager
            .alter_type(
                Type::alter()
                    .name(Alias::new("PaymentStatus"))
                    .add_value(Alias::new("CANCELLED"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Note: Most databases don't support removing enum values directly
        // This is a simplified approach that recreates the enum without CANCELLED

        // First, we need to handle any existing data with CANCELLED status
        // You might want to update these records to another status before running this

        // For PostgreSQL, you would typically need to:
        // 1. Create a new enum type without CANCELLED
        // 2. Update the column to use the new enum type
        // 3. Drop the old enum type

        // This is a basic implementation - adjust based on your needs
        manager
            .drop_type(Type::drop().name(Alias::new("PaymentStatus")).to_owned())
            .await?;

        manager
            .create_type(
                Type::create()
                    .as_enum(Alias::new("PaymentStatus"))
                    .values([
                        Alias::new("FAILED"),
                        Alias::new("PAID"),
                        Alias::new("PENDING"),
                        Alias::new("REFUNDED"),
                    ])
                    .to_owned(),
            )
            .await
    }
}
