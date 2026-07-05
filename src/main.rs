mod db;
mod parser;
mod scraper;
mod types;

use serde_json::json;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

use types::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let started = Instant::now();

    // ── Config ────────────────────────────────────────────────────────
    if std::path::Path::new(".env").exists() {
        dotenvy::dotenv().ok();
    } else {
        eprintln!("note: no .env file found — copy .env.example to .env and add your key");
    }

    let api_key = std::env::var("FIRECRAWL_API_KEY").map_err(|_| {
        "FIRECRAWL_API_KEY not set.\n\
         Copy .env.example to .env and add your Firecrawl API key, or:\n\
         export FIRECRAWL_API_KEY=fc-your-key"
    })?;

    let db_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:halal.db?mode=rwc".to_string());

    // ── Init ──────────────────────────────────────────────────────────
    let client = Arc::new(firecrawl::Client::new(&api_key)?);
    let semaphore = types::semaphore();
    let pool = db::init(&db_url).await?;

    let base = "https://myehalal.halal.gov.my/portal-halal/v1/index.php";
    let data = "ZGlyZWN0b3J5L2luZGV4X2RpcmVjdG9yeTs7Ozs=";
    let categories = types::categories();

    // ── Scrape ────────────────────────────────────────────────────────
    let total_categories = categories.len();
    let mut all_records: Vec<serde_json::Value> = Vec::new();
    let mut cat_summary: Vec<(String, u32, Duration)> = Vec::new();

    for (cat_idx, (code, name, subs)) in categories.iter().enumerate() {
        let cat_start = Instant::now();
        let mut cat_total = 0u32;

        println!(
            "\n┌─ [{}/{}] {name} ({code})",
            cat_idx + 1,
            total_categories
        );

        let sub_pairs: Vec<(&str, &str)> = if subs.is_empty() {
            vec![("", "")]
        } else {
            subs.iter().map(|(c, n)| (*c, *n)).collect()
        };
        let total_subs = sub_pairs.len();

        for (sub_idx, (sub_code, sub_name)) in sub_pairs.iter().enumerate() {
            let label = if sub_code.is_empty() {
                format!("{name} (default)")
            } else {
                format!("{name} › {sub_name}")
            };

            if total_subs > 1 {
                println!("│  ┌─ [{}.{}] {}", cat_idx + 1, sub_idx + 1, label);
            } else {
                println!("│  └─ {label}");
            }

            match scraper::scrape_category(&client, &semaphore, base, data, code, sub_code).await {
                Ok(records) => {
                    let count = records.len() as u32;
                    cat_total += count;
                    for mut r in records {
                        if let Some(obj) = r.as_object_mut() {
                            obj.insert("category_code".into(), json!(code));
                            obj.insert("category_name".into(), json!(name));
                            obj.insert("subcategory_code".into(), json!(sub_code));
                            obj.insert("subcategory_name".into(), json!(sub_name));
                        }
                        all_records.push(r);
                    }
                }
                Err(e) => eprintln!("│    ✗ FAILED: {e}"),
            }
        }

        let elapsed = cat_start.elapsed();
        println!(
            "└─ {name}: {cat_total} records in {:.1}s",
            elapsed.as_secs_f32()
        );
        cat_summary.push((name.to_string(), cat_total, elapsed));
    }

    // ── Save ──────────────────────────────────────────────────────────
    db::insert_all(&pool, &all_records).await?;

    let json = serde_json::to_string_pretty(&all_records)?;
    fs::write("halal_all.json", &json)?;
    println!(
        "Saved halal_all.json ({:.1} MB)",
        json.len() as f32 / 1_000_000.0
    );

    // ── Summary ───────────────────────────────────────────────────────
    let total_elapsed = started.elapsed();
    println!("\n╔══════════════════════════════════════════╗");
    println!(
        "║  DONE: {} records in {:.1}s          ║",
        all_records.len(),
        total_elapsed.as_secs_f32()
    );
    println!("╚══════════════════════════════════════════╝");

    println!("\n── Per-category summary ──");
    for (name, count, dur) in &cat_summary {
        println!("  {name:30} {count:>6} records  {:.1}s", dur.as_secs_f32());
    }

    println!("\n── Sample records ──");
    for r in all_records.iter().take(5) {
        println!(
            "  [{}/{}] #{} | {} | {}",
            r["category_code"].as_str().unwrap_or("?"),
            r["subcategory_code"].as_str().unwrap_or("-"),
            r["bil"],
            r["company"].as_str().unwrap_or("?"),
            r["expiry_date"].as_str().unwrap_or("?")
        );
    }

    Ok(())
}
