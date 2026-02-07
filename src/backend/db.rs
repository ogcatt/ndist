#[cfg(feature = "server")]
use sea_orm::{Database, DatabaseConnection, DbErr};
#[cfg(feature = "server")]
use std::env;
#[cfg(feature = "server")]
use tokio::sync::OnceCell;

#[cfg(feature = "server")]
static DB: OnceCell<DatabaseConnection> = OnceCell::const_new();

#[cfg(feature = "server")]
async fn init_db() -> DatabaseConnection {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set.");

    let mut opt = sea_orm::ConnectOptions::new(database_url);

    // Configure connection pool settings
    opt.max_connections(10) // Limit connections per instance
        .min_connections(1)
        .connect_timeout(std::time::Duration::from_secs(8))
        .acquire_timeout(std::time::Duration::from_secs(8))
        .idle_timeout(std::time::Duration::from_secs(8))
        .max_lifetime(std::time::Duration::from_secs(8))
        .sqlx_logging(true);

    Database::connect(opt)
        .await
        .expect("COULD NOT CONNECT TO DATABASE WITH SEAORM")
}

#[cfg(feature = "server")]
pub async fn get_db() -> &'static DatabaseConnection {
    DB.get_or_init(init_db).await
}
