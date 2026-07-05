use serde_json::json;

/// Parse a markdown table into structured records.
pub fn parse_table(md: &str, page: u32) -> Vec<serde_json::Value> {
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

/// Extract total page count from a markdown line containing "Total Record : N - Page X From Y".
pub fn extract_total_pages(md: &str) -> u32 {
    md.lines()
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
        .unwrap_or(1)
}

/// Split the "Nama & Alamat Syarikat" cell into company name and address.
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

/// Remove HTML tags and normalize whitespace.
fn clean_html(s: &str) -> String {
    s.replace("<br>", ", ")
        .replace(['<', '>'], "")
        .split(',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}
