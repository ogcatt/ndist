use sea_orm_migration::prelude::*;
use sea_orm_migration::prelude::extension::postgres::Type;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add the new EXPIRED value to the PaymentStatus enum
        manager
            .alter_type(
                Type::alter()
                    .name(Alias::new("PaymentStatus"))
                    .add_value(Alias::new("EXPIRED"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Note: Most databases don't support removing enum values directly
        // This is a simplified rollback that recreates the enum without EXPIRED
        // You may need to handle existing data migration if there are records with EXPIRED status

        // First, create a temporary enum without EXPIRED
        manager
            .create_type(
                Type::create()
                    .as_enum(Alias::new("PaymentStatus_temp"))
                    .values([
                        Alias::new("CANCELLED"),
                        Alias::new("FAILED"),
                        Alias::new("PAID"),
                        Alias::new("PENDING"),
                        Alias::new("REFUNDED"),
                    ])
                    .to_owned(),
            )
            .await?;

        // You would need to update any columns using this enum to use the temp enum
        // This is database-specific and depends on your table structure
        // Example (uncomment and modify based on your actual table):
        /*
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("your_table_name"))
                    .modify_column(
                        ColumnDef::new(Alias::new("status"))
                            .custom(Alias::new("PaymentStatus_temp"))
                            .not_null()
                    )
                    .to_owned(),
            )
            .await?;
        */

        // Drop the original enum and rename the temp one
        manager
            .drop_type(Type::drop().name(Alias::new("PaymentStatus")).to_owned())
            .await?;

        manager
            .alter_type(
                Type::alter()
                    .name(Alias::new("PaymentStatus_temp"))
                    .rename_to(Alias::new("PaymentStatus"))
                    .to_owned(),
            )
            .await
    }
}
