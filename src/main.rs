mod db;
mod parser;
mod scraper;
mod types;

use std::sync::Arc;
use std::time::Instant;

use types::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let started = Instant::now();

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

    let client = Arc::new(firecrawl::Client::new(&api_key)?);
    let semaphore = types::semaphore();
    let pool = db::init(&db_url).await?;

    let base = "https://myehalal.halal.gov.my/portal-halal/v1/index.php";
    let company_strategies = types::company_strategies();
    let other_strategies = types::other_strategies();

    db::seed_categories(&pool, &company_strategies).await?;
    db::seed_categories(&pool, &other_strategies).await?;

    let mut total_companies = 0usize;
    let mut total_others = 0usize;
    let max_pages = u32::MAX;

    // ── Phase 1: Companies (CO) ───────────────────────────────────────
    println!("\n═══ PHASE 1: COMPANIES (CO) ═══");
    for (idx, s) in company_strategies.iter().enumerate() {
        println!(
            "\n┌─ [{}/{}] {} ({})",
            idx + 1,
            company_strategies.len(),
            s.category_name,
            s.category_code
        );

        match scraper::scrape_subcategory(&client, &semaphore, base, s, max_pages).await {
            Ok(records) => {
                let n = db::insert_companies(&pool, &records, s.category_code).await?;
                total_companies += n;
                println!("└─ {n} companies → DB");
            }
            Err(e) => eprintln!("└─ ✗ {e}"),
        }
    }

    // ── Phase 2: Products & others ────────────────────────────────────
    println!("\n═══ PHASE 2: PRODUCTS & OTHERS ═══");
    for (idx, s) in other_strategies.iter().enumerate() {
        println!(
            "\n┌─ [{}/{}] {} › {} ({})",
            idx + 1,
            other_strategies.len(),
            s.category_name,
            s.sub_name,
            s.sub_code
        );

        match scraper::scrape_subcategory(&client, &semaphore, base, s, max_pages).await {
            Ok(records) => {
                let n = db::insert_products(&pool, &records, s.category_code, s.sub_code).await?;
                total_others += n;
                println!("└─ {n} listings → DB");
            }
            Err(e) => eprintln!("└─ ✗ {e}"),
        }
    }

    // ── Summary ───────────────────────────────────────────────────────
    let elapsed = started.elapsed();
    println!("\n╔══════════════════════════════════════════╗");
    println!(
        "║  DONE: {} companies + {} others  ║",
        total_companies, total_others
    );
    println!(
        "║  in {:.1}s                          ║",
        elapsed.as_secs_f32()
    );
    println!("╚══════════════════════════════════════════╝");

    println!("\n── Sample companies ──");
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT name, address FROM companies ORDER BY RANDOM() LIMIT 3")
            .fetch_all(&pool)
            .await?;
    for (name, addr) in &rows {
        println!("  {name} — {addr}");
    }

    println!("\n── Sample others ──");
    let rows: Vec<(String, Option<String>, String)> = sqlx::query_as(
        "SELECT name, address, subcategory_code FROM products ORDER BY RANDOM() LIMIT 3",
    )
    .fetch_all(&pool)
    .await?;
    for (name, addr, code) in &rows {
        println!("  [{code}] {name} — {}", addr.as_deref().unwrap_or("-"));
    }

    Ok(())
}
