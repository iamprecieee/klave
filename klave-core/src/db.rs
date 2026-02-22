use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;

use crate::error::KlaveError;

pub async fn init_pool(database_url: &str) -> Result<SqlitePool, KlaveError> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
