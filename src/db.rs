use serde_json::Value;
use sqlx::sqlite::SqlitePool;

use crate::types::{Error, SubStrategy};

pub async fn init(db_url: &str) -> Result<SqlitePool, Error> {
    let pool = SqlitePool::connect(db_url).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS categories (
            code TEXT PRIMARY KEY,
            name TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS companies (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            address TEXT NOT NULL,
            expiry_date TEXT NOT NULL,
            category_code TEXT NOT NULL REFERENCES categories(code),
            bil INTEGER NOT NULL,
            scraped_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(category_code, name)
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS products (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            address TEXT,
            expiry_date TEXT,
            category_code TEXT NOT NULL REFERENCES categories(code),
            subcategory_code TEXT NOT NULL,
            bil INTEGER NOT NULL,
            scraped_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(category_code, subcategory_code, name)
        )",
    )
    .execute(&pool)
    .await?;

    println!("  DB ready: {db_url}");
    Ok(pool)
}

pub async fn seed_categories(pool: &SqlitePool, strategies: &[SubStrategy]) -> Result<(), Error> {
    for s in strategies {
        sqlx::query("INSERT OR IGNORE INTO categories (code, name) VALUES (?, ?)")
            .bind(s.category_code)
            .bind(s.category_name)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn insert_companies(
    pool: &SqlitePool,
    records: &[Value],
    category_code: &str,
) -> Result<usize, Error> {
    if records.is_empty() {
        return Ok(0);
    }
    let mut tx = pool.begin().await?;
    for r in records {
        sqlx::query(
            "INSERT OR REPLACE INTO companies (name, address, expiry_date, category_code, bil, scraped_at)
             VALUES (?, ?, ?, ?, ?, datetime('now'))",
        )
        .bind(r["name"].as_str().unwrap_or(""))
        .bind(r["address"].as_str().unwrap_or(""))
        .bind(r["expiry_date"].as_str().unwrap_or(""))
        .bind(category_code)
        .bind(r["bil"].as_i64().unwrap_or(0))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(records.len())
}

pub async fn insert_products(
    pool: &SqlitePool,
    records: &[Value],
    category_code: &str,
    subcategory_code: &str,
) -> Result<usize, Error> {
    if records.is_empty() {
        return Ok(0);
    }
    let mut tx = pool.begin().await?;
    for r in records {
        sqlx::query(
            "INSERT OR REPLACE INTO products (name, address, expiry_date, category_code, subcategory_code, bil, scraped_at)
             VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
        )
        .bind(r["name"].as_str().unwrap_or(""))
        .bind(r["address"].as_str().unwrap_or(""))
        .bind(r["expiry_date"].as_str().unwrap_or(""))
        .bind(category_code)
        .bind(subcategory_code)
        .bind(r["bil"].as_i64().unwrap_or(0))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(records.len())
}
