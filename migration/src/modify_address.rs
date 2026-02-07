use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add the orderId column
        manager
            .alter_table(
                Table::alter()
                    .table(Address::Table)
                    .add_column(ColumnDef::new(Address::OrderId).text().not_null())
                    .to_owned(),
            )
            .await?;

        // Make customerId nullable
        manager
            .alter_table(
                Table::alter()
                    .table(Address::Table)
                    .modify_column(ColumnDef::new(Address::CustomerId).text().null())
                    .to_owned(),
            )
            .await?;

        // Add address_line_1 column
        manager
            .alter_table(
                Table::alter()
                    .table(Address::Table)
                    .add_column(ColumnDef::new(Address::AddressLine1).text().not_null())
                    .to_owned(),
            )
            .await?;

        // Add address_line_2 column (nullable)
        manager
            .alter_table(
                Table::alter()
                    .table(Address::Table)
                    .add_column(ColumnDef::new(Address::AddressLine2).text().null())
                    .to_owned(),
            )
            .await?;

        // Rename state column to province
        manager
            .alter_table(
                Table::alter()
                    .table(Address::Table)
                    .rename_column(Address::State, Address::Province)
                    .to_owned(),
            )
            .await?;

        // Drop the street column
        manager
            .alter_table(
                Table::alter()
                    .table(Address::Table)
                    .drop_column(Address::Street)
                    .to_owned(),
            )
            .await?;

        // Add foreign key constraint for orderId
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_address_order_id")
                    .from(Address::Table, Address::OrderId)
                    .to(Order::Table, Order::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Restrict)
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
                    .name("fk_address_order_id")
                    .table(Address::Table)
                    .to_owned(),
            )
            .await?;

        // Add back street column
        manager
            .alter_table(
                Table::alter()
                    .table(Address::Table)
                    .add_column(ColumnDef::new(Address::Street).text().not_null())
                    .to_owned(),
            )
            .await?;

        // Rename province back to state
        manager
            .alter_table(
                Table::alter()
                    .table(Address::Table)
                    .rename_column(Address::Province, Address::State)
                    .to_owned(),
            )
            .await?;

        // Drop address_line_2 column
        manager
            .alter_table(
                Table::alter()
                    .table(Address::Table)
                    .drop_column(Address::AddressLine2)
                    .to_owned(),
            )
            .await?;

        // Drop address_line_1 column
        manager
            .alter_table(
                Table::alter()
                    .table(Address::Table)
                    .drop_column(Address::AddressLine1)
                    .to_owned(),
            )
            .await?;

        // Make customerId not nullable
        manager
            .alter_table(
                Table::alter()
                    .table(Address::Table)
                    .modify_column(ColumnDef::new(Address::CustomerId).text().not_null())
                    .to_owned(),
            )
            .await?;

        // Drop the orderId column
        manager
            .alter_table(
                Table::alter()
                    .table(Address::Table)
                    .drop_column(Address::OrderId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Address {
    #[sea_orm(iden = "Address")]
    Table,
    #[sea_orm(iden = "orderId")]
    OrderId,
    #[sea_orm(iden = "customerId")]
    CustomerId,
    #[sea_orm(iden = "street")]
    Street,
    #[sea_orm(iden = "addressLine1")]
    AddressLine1,
    #[sea_orm(iden = "addressLine2")]
    AddressLine2,
    State,
    Province,
}

#[derive(DeriveIden)]
enum Order {
    #[sea_orm(iden = "Order")]
    Table,
    Id,
}
