use serde_json::Value;
use sqlx::sqlite::SqlitePool;
use std::time::Instant;

use crate::types::Error;

/// Connect to the database and ensure the schema exists.
/// Clears previous data on each run.
pub async fn init(db_url: &str) -> Result<SqlitePool, Error> {
    let pool = SqlitePool::connect(db_url).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS companies (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            bil INTEGER NOT NULL,
            company TEXT NOT NULL,
            address TEXT NOT NULL,
            expiry_date TEXT NOT NULL,
            category_code TEXT NOT NULL,
            category_name TEXT NOT NULL,
            subcategory_code TEXT NOT NULL DEFAULT '',
            subcategory_name TEXT NOT NULL DEFAULT '',
            page INTEGER NOT NULL,
            scraped_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query("DELETE FROM companies").execute(&pool).await?;

    println!("  DB ready: {db_url}");
    Ok(pool)
}

/// Batch-insert all records in a single transaction.
pub async fn insert_all(pool: &SqlitePool, records: &[Value]) -> Result<(), Error> {
    println!("\n── Writing to database...");
    let started = Instant::now();

    let mut tx = pool.begin().await?;
    for r in records {
        sqlx::query(
            "INSERT INTO companies (bil, company, address, expiry_date, category_code, category_name, subcategory_code, subcategory_name, page)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(r["bil"].as_i64().unwrap_or(0))
        .bind(r["company"].as_str().unwrap_or(""))
        .bind(r["address"].as_str().unwrap_or(""))
        .bind(r["expiry_date"].as_str().unwrap_or(""))
        .bind(r["category_code"].as_str().unwrap_or(""))
        .bind(r["category_name"].as_str().unwrap_or(""))
        .bind(r["subcategory_code"].as_str().unwrap_or(""))
        .bind(r["subcategory_name"].as_str().unwrap_or(""))
        .bind(r["page"].as_i64().unwrap_or(0))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    println!(
        "  DB write: {} rows in {:.1}s",
        records.len(),
        started.elapsed().as_secs_f32()
    );
    Ok(())
}
