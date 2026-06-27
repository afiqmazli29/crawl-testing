use firecrawl::{Client, Format, ScrapeOptions};
use serde_json::json;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::sleep;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Category<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str)]);

const MAX_CONCURRENT: usize = 5;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let client = Arc::new(Client::new("fc-cf9e4e0cd66647babcbc6932f424737f")?);
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

    let mut all_records: Vec<serde_json::Value> = Vec::new();

    for (code, name, subs) in categories {
        println!("\n══════ {name} ({code}) ══════");

        let sub_pairs: Vec<(&str, &str)> = if subs.is_empty() {
            vec![("", "")]
        } else {
            subs.iter().map(|(c, n)| (*c, *n)).collect()
        };

        for (sub_code, sub_name) in &sub_pairs {
            let label = if sub_code.is_empty() {
                format!("{name} (default)")
            } else {
                format!("{name} › {sub_name}")
            };
            println!("  ── {label} ──");

            match scrape_category(&client, &semaphore, base, data, code, sub_code).await {
                Ok(records) => {
                    let count = records.len();
                    for mut r in records {
                        if let Some(obj) = r.as_object_mut() {
                            obj.insert("category_code".into(), json!(code));
                            obj.insert("category_name".into(), json!(name));
                            obj.insert("subcategory_code".into(), json!(sub_code));
                            obj.insert("subcategory_name".into(), json!(sub_name));
                        }
                        all_records.push(r);
                    }
                    println!("    → {count} records");
                }
                Err(e) => {
                    eprintln!("    ✗ {e}");
                }
            }
        }
    }

    // ── Output ───────────────────────────────────────────────────────
    println!("\n══════ DONE: {} records ══════", all_records.len());

    let json = serde_json::to_string_pretty(&all_records)?;
    fs::write("halal_all.json", &json)?;
    println!("Saved halal_all.json");

    println!("\n--- Sample ---");
    for r in all_records.iter().take(10) {
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
    // Determine URL param name for subcategory
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

            let mut records = parse_table(&md, 1);

            if total_pages > 1 {
                // Fetch remaining pages concurrently
                let mut set = JoinSet::new();
                for page in 2..=total_pages {
                    let url = format!("{base}?data={data}&category={cat_code}{sp}&page={page}");
                    let c = Arc::clone(client);
                    let s = Arc::clone(semaphore);
                    set.spawn(async move {
                        let doc = scrape_with_retry(&c, &s, &url, 3).await?;
                        Ok::<_, Error>(parse_table(&doc.markdown.unwrap_or_default(), page))
                    });
                }
                while let Some(result) = set.join_next().await {
                    match result? {
                        Ok(mut page_records) => records.append(&mut page_records),
                        Err(e) => eprintln!("      ✗ concurrent page: {e}"),
                    }
                }
            }

            return Ok(records);
        }
    }

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
            Ok(doc) => {
                if attempt > 1 {
                    eprintln!("      (ok retry {attempt})");
                }
                return Ok(doc);
            }
            Err(e) => {
                let msg = e.to_string();
                let retryable = msg.contains("TUNNEL_CONNECTION_FAILED")
                    || msg.contains("proxy error")
                    || msg.contains("timeout")
                    || msg.contains("Timed out");
                if retryable && attempt < max_retries {
                    let delay = Duration::from_secs(2u64.pow(attempt));
                    eprintln!("      ⚠ retry in {}s", delay.as_secs());
                    sleep(delay).await;
                } else {
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
