use sea_orm_migration::prelude::*;
use sea_orm_migration::prelude::extension::postgres::Type;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ShippingOption enum type already exists in database, so we skip creation

        // Update Order table
        manager
            .alter_table(
                Table::alter()
                    .table(Order::Table)
                    // Remove old columns
                    .drop_column(Order::ShippingAddressId)
                    .drop_column(Order::BillingAddressId)
                    .drop_column(Order::RegionId)
                    .drop_column(Order::PaymentId)
                    .drop_column(Order::PlacedAt)
                    // Add new columns
                    .add_column(
                        ColumnDef::new(Order::CustomerEmail)
                            .string()
                            .not_null()
                    )
                    .add_column(
                        ColumnDef::new(Order::BillingCountry)
                            .string()
                            .not_null()
                    )
                    .add_column(
                        ColumnDef::new(Order::ShippingOption)
                            .enumeration(ShippingOption::Table, [
                                ShippingOption::Tracked,
                                ShippingOption::Express,
                                ShippingOption::TrackedUS,
                            ])
                            .not_null()
                    )
                    .add_column(
                        ColumnDef::new(Order::SubtotalUsd)
                            .double()
                            .not_null()
                    )
                    .add_column(
                        ColumnDef::new(Order::ShippingUsd)
                            .double()
                            .not_null()
                    )
                    .add_column(
                        ColumnDef::new(Order::OrderWeight)
                            .double()
                            .not_null()
                    )
                    .add_column(
                        ColumnDef::new(Order::RefundComment)
                            .string()
                            .null()
                    )
                    .to_owned(),
            )
            .await?;

        // Update Payment table
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    // Add order_id column
                    .add_column(
                        ColumnDef::new(Payment::OrderId)
                            .string()
                            .not_null()
                    )
                    // Drop old ref column and add new processor_ref
                    .drop_column(Payment::Ref)
                    .drop_column(Payment::RawData)
                    .add_column(
                        ColumnDef::new(Payment::ProcessorRef)
                            .string()
                            .null()
                    )
                    .to_owned(),
            )
            .await?;

        // Add foreign key constraint for Payment -> Order
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_payment_order_id")
                    .from(Payment::Table, Payment::OrderId)
                    .to(Order::Table, Order::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop foreign key constraint
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_payment_order_id")
                    .table(Payment::Table)
                    .to_owned(),
            )
            .await?;

        // Revert Payment table changes
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .drop_column(Payment::OrderId)
                    .drop_column(Payment::ProcessorRef)
                    .add_column(
                        ColumnDef::new(Payment::Ref)
                            .string()
                            .null()
                    )
                    .add_column(
                        ColumnDef::new(Payment::RawData)
                            .json_binary()
                            .null()
                    )
                    .to_owned(),
            )
            .await?;

        // Revert Order table changes
        manager
            .alter_table(
                Table::alter()
                    .table(Order::Table)
                    // Remove new columns
                    .drop_column(Order::CustomerEmail)
                    .drop_column(Order::BillingCountry)
                    .drop_column(Order::ShippingOption)
                    .drop_column(Order::SubtotalUsd)
                    .drop_column(Order::ShippingUsd)
                    .drop_column(Order::OrderWeight)
                    .drop_column(Order::RefundComment)
                    // Add back old columns
                    .add_column(
                        ColumnDef::new(Order::ShippingAddressId)
                            .string()
                            .not_null()
                    )
                    .add_column(
                        ColumnDef::new(Order::BillingAddressId)
                            .string()
                            .not_null()
                    )
                    .add_column(
                        ColumnDef::new(Order::RegionId)
                            .string()
                            .not_null()
                    )
                    .add_column(
                        ColumnDef::new(Order::PaymentId)
                            .string()
                            .null()
                    )
                    .add_column(
                        ColumnDef::new(Order::PlacedAt)
                            .timestamp()
                            .not_null()
                    )
                    .to_owned(),
            )
            .await?;

        // Don't drop ShippingOption enum type since it existed before this migration
        // and may be used by other tables or migrations

        Ok(())
    }
}

#[derive(Iden)]
enum Order {
    #[iden = "Order"]
    Table,
    Id,
    #[iden = "customerId"]
    CustomerId,
    #[iden = "customerEmail"]
    CustomerEmail,
    #[iden = "billingCountry"]
    BillingCountry,
    #[iden = "shippingOption"]
    ShippingOption,
    #[iden = "subtotalUsd"]
    SubtotalUsd,
    #[iden = "shippingUsd"]
    ShippingUsd,
    #[iden = "orderWeight"]
    OrderWeight,
    #[iden = "refundComment"]
    RefundComment,
    // Old columns being removed
    #[iden = "shippingAddressId"]
    ShippingAddressId,
    #[iden = "billingAddressId"]
    BillingAddressId,
    #[iden = "regionId"]
    RegionId,
    #[iden = "paymentId"]
    PaymentId,
    #[iden = "placedAt"]
    PlacedAt,
}

#[derive(Iden)]
enum Payment {
    #[iden = "Payment"]
    Table,
    Id,
    #[iden = "orderId"]
    OrderId,
    Method,
    #[iden = "processorRef"]
    ProcessorRef,
    Status,
    #[iden = "amountUsd"]
    AmountUsd,
    #[iden = "paidAt"]
    PaidAt,
    #[iden = "createdAt"]
    CreatedAt,
    #[iden = "updatedAt"]
    UpdatedAt,
    // Old columns being removed
    Ref,
    #[iden = "rawData"]
    RawData,
}

#[derive(Iden)]
enum ShippingOption {
    Table,
    #[iden = "TRACKED"]
    Tracked,
    #[iden = "EXPRESS"]
    Express,
    #[iden = "TRACKED_US"]
    TrackedUS,
}
