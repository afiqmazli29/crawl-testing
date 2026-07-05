# Malaysia Halal Directory Scraper

Scrapes the [Malaysia Halal Portal](https://myehalal.halal.gov.my/) public directory — extracting company listings across all categories and subcategories into structured JSON.

## Quick start

```bash
# Copy the example env file and add your Firecrawl API key
cp .env.example .env
# Edit .env with your key, then:

cargo run
```

Output: `halal_all.json` with all records across every category.

## Data fields

| Field | Description |
|-------|-------------|
| `bil` | Row number on the page |
| `company` | Company name |
| `address` | Full address |
| `expiry_date` | Halal certification expiry date(s) |
| `category_code` | Main category code (e.g. `BG`, `FM`) |
| `category_name` | Main category name |
| `subcategory_code` | Subcategory code (e.g. `CO`, `BG`) |
| `subcategory_name` | Subcategory name |
| `page` | Source page number |

## Categories scraped

| Code | Name | Subcategories |
|------|------|---------------|
| BG | Barang Gunaan | Syarikat, Barang Gunaan |
| FM | Farmaseutikal | Syarikat, Farmaseutikal |
| IN | International | — |
| KO | Kosmetik & Dandanan | Syarikat, Kosmetik |
| MD | Peranti Perubatan | Syarikat, Peranti Perubatan |
| OEM | OEM | Syarikat, OEM |
| PE | Premis Makanan | Syarikat, Hotel & Resort, Premis Makanan |
| PL | Logistik | Syarikat |
| PR | Produk Makanan/Minuman | Syarikat, Produk |
| PS | Rumah Sembelihan | Syarikat, Rumah Sembelih |

## Configuration

- **Concurrency**: 5 parallel requests (`MAX_CONCURRENT` constant)
- **Retries**: 3 attempts with exponential backoff for transient errors
- **Timeout**: 90 seconds per scrape request
