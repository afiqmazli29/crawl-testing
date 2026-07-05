use firecrawl::{Client, Format, ScrapeOptions};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::sleep;

use crate::parser;
use crate::types::Error;

/// Scrape all pages for a category+subcategory combination.
pub async fn scrape_category(
    client: &Arc<Client>,
    semaphore: &Arc<Semaphore>,
    base: &str,
    data: &str,
    cat_code: &str,
    sub_code: &str,
) -> Result<Vec<serde_json::Value>, Error> {
    let param_candidates: &[&str] = if sub_code.is_empty() {
        &[""]
    } else {
        &["subcategory", "menu", "sub", "type"]
    };

    for pname in param_candidates {
        let sp = if pname.is_empty() {
            String::new()
        } else {
            format!("&{pname}={sub_code}")
        };
        let url1 = format!("{base}?data={data}&category={cat_code}{sp}");

        let t0 = Instant::now();
        let doc = scrape_with_retry(client, semaphore, &url1, 3).await?;
        let md = doc.markdown.unwrap_or_default();

        if md.contains("Total Record") {
            let total_pages = parser::extract_total_pages(&md);

            let page1_count = parser::parse_table(&md, 1).len();
            println!(
                "│    page 1/{total_pages} ✓  {page1_count} records  {:.1}s",
                t0.elapsed().as_secs_f32()
            );

            let mut records = parser::parse_table(&md, 1);

            if total_pages > 1 {
                let more = fetch_remaining_pages(
                    client,
                    semaphore,
                    base,
                    data,
                    cat_code,
                    &sp,
                    total_pages,
                )
                .await?;
                records.extend(more);
            }

            return Ok(records);
        }
    }

    println!("│    (no records)");
    Ok(vec![])
}

/// Fetch pages 2..total_pages concurrently.
async fn fetch_remaining_pages(
    client: &Arc<Client>,
    semaphore: &Arc<Semaphore>,
    base: &str,
    data: &str,
    cat_code: &str,
    sub_param: &str,
    total_pages: u32,
) -> Result<Vec<serde_json::Value>, Error> {
    let done = Arc::new(AtomicU32::new(1)); // page 1 already done
    let failed = Arc::new(AtomicU32::new(0));
    let mut set = JoinSet::new();
    let mut records = Vec::new();

    for page in 2..=total_pages {
        let url = format!("{base}?data={data}&category={cat_code}{sub_param}&page={page}");
        let c = Arc::clone(client);
        let s = Arc::clone(semaphore);
        let d = Arc::clone(&done);

        set.spawn(async move {
            let t = Instant::now();
            let doc = scrape_with_retry(&c, &s, &url, 3).await?;
            let page_records = parser::parse_table(&doc.markdown.unwrap_or_default(), page);
            let n = page_records.len();
            let done_now = d.fetch_add(1, Ordering::Relaxed) + 1;
            println!(
                "│    page {page}/{total_pages} ✓  {n} records  {:.1}s  [{done_now}/{total_pages}]",
                t.elapsed().as_secs_f32()
            );
            Ok::<_, Error>(page_records)
        });
    }

    while let Some(result) = set.join_next().await {
        match result? {
            Ok(mut page_records) => records.append(&mut page_records),
            Err(e) => {
                failed.fetch_add(1, Ordering::Relaxed);
                eprintln!("│    ✗ page failed: {e}");
            }
        }
    }

    let failed_count = failed.load(Ordering::Relaxed);
    if failed_count > 0 {
        eprintln!("│    ⚠ {failed_count}/{} page(s) failed", total_pages - 1);
    }

    Ok(records)
}

/// Scrape a URL with retry on transient errors.
async fn scrape_with_retry(
    client: &Client,
    semaphore: &Semaphore,
    url: &str,
    max_retries: u32,
) -> Result<firecrawl::Document, Error> {
    for attempt in 1..=max_retries {
        let _permit = semaphore.acquire().await?;

        match client
            .scrape(
                url,
                ScrapeOptions {
                    formats: Some(vec![Format::Markdown]),
                    timeout: Some(90_000),
                    ..Default::default()
                },
            )
            .await
        {
            Ok(doc) => return Ok(doc),
            Err(e) => {
                let msg = e.to_string();
                let retryable = msg.contains("TUNNEL_CONNECTION_FAILED")
                    || msg.contains("proxy error")
                    || msg.contains("timeout")
                    || msg.contains("Timed out");
                if retryable && attempt < max_retries {
                    let delay = Duration::from_secs(2u64.pow(attempt));
                    if attempt == 1 {
                        eprint!("│    ⚠ retrying");
                    }
                    eprint!(" {attempt}/{max_retries}");
                    sleep(delay).await;
                } else {
                    if attempt > 1 {
                        eprintln!();
                    }
                    return Err(e.into());
                }
            }
        }
    }
    Err("all retries exhausted".into())
}
