use firecrawl::{Client, Format, ScrapeOptions};
use serde_json::json;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::sleep;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Category<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str)]);

const MAX_CONCURRENT: usize = 5;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let started = Instant::now();

    // Load .env file if present
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

    let client = Arc::new(Client::new(&api_key)?);
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));

    let base = "https://myehalal.halal.gov.my/portal-halal/v1/index.php";
    let data = "ZGlyZWN0b3J5L2luZGV4X2RpcmVjdG9yeTs7Ozs=";

    let categories: &[Category] = &[
        (
            "BG",
            "Barang Gunaan",
            &[("CO", "Syarikat"), ("BG", "Barang Gunaan")],
        ),
        (
            "FM",
            "Farmaseutikal",
            &[("CO", "Syarikat"), ("FM", "Farmaseutikal")],
        ),
        ("IN", "International", &[]),
        (
            "KO",
            "Kosmetik & Dandanan",
            &[("CO", "Syarikat"), ("KO", "Kosmetik")],
        ),
        (
            "MD",
            "Peranti Perubatan",
            &[("CO", "Syarikat"), ("MD", "Peranti Perubatan")],
        ),
        ("OEM", "OEM", &[("CO", "Syarikat"), ("OEM", "OEM")]),
        (
            "PE",
            "Premis Makanan",
            &[
                ("CO", "Syarikat"),
                ("HO", "Hotel & Resort"),
                ("PE", "Premis Makanan"),
            ],
        ),
        ("PL", "Logistik", &[("CO", "Syarikat")]),
        (
            "PR",
            "Produk Makanan/Minuman",
            &[("CO", "Syarikat"), ("PR", "Produk")],
        ),
        (
            "PS",
            "Rumah Sembelihan",
            &[("CO", "Syarikat"), ("RS", "Rumah Sembelih")],
        ),
    ];

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
                println!(
                    "│  ┌─ [{}/{}.{}] {}",
                    cat_idx + 1,
                    sub_idx + 1,
                    total_subs,
                    label
                );
            } else {
                println!("│  └─ {label}");
            }

            match scrape_category(&client, &semaphore, base, data, code, sub_code).await {
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
                Err(e) => {
                    eprintln!("│    ✗ FAILED: {e}");
                }
            }
        }

        let elapsed = cat_start.elapsed();
        println!(
            "└─ {name}: {cat_total} records in {:.1}s",
            elapsed.as_secs_f32()
        );
        cat_summary.push((name.to_string(), cat_total, elapsed));
    }

    // ── Output ───────────────────────────────────────────────────────
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

    let json = serde_json::to_string_pretty(&all_records)?;
    fs::write("halal_all.json", &json)?;
    println!(
        "\nSaved halal_all.json ({:.1} MB)",
        json.len() as f32 / 1_000_000.0
    );

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

async fn scrape_category(
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
            let total_pages: u32 = md
                .lines()
                .find_map(|line| {
                    if line.contains("Total Record") {
                        line.split("From")
                            .nth(1)
                            .map(|s| s.chars().filter(|c| c.is_ascii_digit()).collect::<String>())
                    } else {
                        None
                    }
                })
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);

            let page1_count = parse_table(&md, 1).len();
            println!(
                "│    page 1/{total_pages} ✓  {page1_count} records  {:.1}s",
                t0.elapsed().as_secs_f32()
            );

            let mut records = parse_table(&md, 1);

            if total_pages > 1 {
                let done = Arc::new(AtomicU32::new(1)); // page 1 already done
                let failed = Arc::new(AtomicU32::new(0));
                let mut set = JoinSet::new();

                for page in 2..=total_pages {
                    let url = format!("{base}?data={data}&category={cat_code}{sp}&page={page}");
                    let c = Arc::clone(client);
                    let s = Arc::clone(semaphore);
                    let d = Arc::clone(&done);

                    set.spawn(async move {
                        let t = Instant::now();
                        let doc = scrape_with_retry(&c, &s, &url, 3).await?;
                        let page_records =
                            parse_table(&doc.markdown.unwrap_or_default(), page);
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
            }

            return Ok(records);
        }
    }

    println!("│    (no records)");
    Ok(vec![])
}

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
                        eprintln!(); // close retry line
                    }
                    return Err(e.into());
                }
            }
        }
    }
    Err("all retries exhausted".into())
}

fn parse_table(md: &str, page: u32) -> Vec<serde_json::Value> {
    let mut records = Vec::new();
    let mut in_table = false;

    for line in md.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("| ---") || trimmed.starts_with("|---") {
            in_table = true;
            continue;
        }
        if trimmed.starts_with('|') && trimmed.contains("Bil") && trimmed.contains("Nama") {
            continue;
        }
        if in_table && (!trimmed.starts_with('|') || trimmed.is_empty()) {
            in_table = false;
            continue;
        }
        if in_table && trimmed.starts_with('|') {
            let cells: Vec<&str> = trimmed
                .split('|')
                .map(|c| c.trim())
                .filter(|c| !c.is_empty())
                .collect();
            if cells.len() >= 3 {
                let bil: i64 = cells[0].parse().unwrap_or(0);
                if bil == 0 {
                    continue;
                }
                let (company, address) = split_company_address(cells[1]);
                let expiry = clean_html(cells[2]);
                records.push(json!({
                    "bil": bil,
                    "company": company.trim().to_string(),
                    "address": address.trim().to_string(),
                    "expiry_date": expiry.trim().to_string(),
                    "page": page,
                }));
            }
        }
    }
    records
}

fn split_company_address(cell: &str) -> (String, String) {
    let parts: Vec<&str> = cell
        .split("<br>")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return (String::new(), String::new());
    }
    (parts[0].to_string(), parts[1..].join(", "))
}

fn clean_html(s: &str) -> String {
    s.replace("<br>", ", ")
        .replace(['<', '>'], "")
        .split(',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}
