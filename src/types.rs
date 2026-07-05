use std::sync::Arc;
use tokio::sync::Semaphore;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub const MAX_CONCURRENT: usize = 5;

/// A subcategory to scrape — CO (companies) goes first, then the rest.
pub struct SubStrategy {
    pub category_code: &'static str,
    pub category_name: &'static str,
    pub data_param: &'static str,
    pub sub_code: &'static str,
    pub sub_name: &'static str,
}

/// Phase 1: CO (Syarikat/companies) — scraped first across all categories.
pub fn company_strategies() -> Vec<SubStrategy> {
    let d = "ZGlyZWN0b3J5L2luZGV4X2RpcmVjdG9yeTs7Ozs=";

    vec![
        SubStrategy {
            category_code: "BG",
            category_name: "Barang Gunaan",
            data_param: d,
            sub_code: "CO",
            sub_name: "Syarikat",
        },
        SubStrategy {
            category_code: "FM",
            category_name: "Farmaseutikal",
            data_param: d,
            sub_code: "CO",
            sub_name: "Syarikat",
        },
        SubStrategy {
            category_code: "KO",
            category_name: "Kosmetik & Dandanan",
            data_param: d,
            sub_code: "CO",
            sub_name: "Syarikat",
        },
        SubStrategy {
            category_code: "MD",
            category_name: "Peranti Perubatan",
            data_param: d,
            sub_code: "CO",
            sub_name: "Syarikat",
        },
        SubStrategy {
            category_code: "OEM",
            category_name: "OEM",
            data_param: d,
            sub_code: "CO",
            sub_name: "Syarikat",
        },
        SubStrategy {
            category_code: "PE",
            category_name: "Premis Makanan",
            data_param: d,
            sub_code: "CO",
            sub_name: "Syarikat",
        },
        SubStrategy {
            category_code: "PL",
            category_name: "Logistik",
            data_param: d,
            sub_code: "CO",
            sub_name: "Syarikat",
        },
        SubStrategy {
            category_code: "PR",
            category_name: "Produk Makanan/Minuman",
            data_param: d,
            sub_code: "CO",
            sub_name: "Syarikat",
        },
        SubStrategy {
            category_code: "PS",
            category_name: "Rumah Sembelihan",
            data_param: d,
            sub_code: "CO",
            sub_name: "Syarikat",
        },
    ]
}

/// Phase 2: Non-CO subcategories — scraped after all companies are done.
pub fn other_strategies() -> Vec<SubStrategy> {
    let d = "ZGlyZWN0b3J5L2luZGV4X2RpcmVjdG9yeTs7Ozs=";

    vec![
        // Products
        SubStrategy {
            category_code: "BG",
            category_name: "Barang Gunaan",
            data_param: d,
            sub_code: "BG",
            sub_name: "Barang Gunaan",
        },
        SubStrategy {
            category_code: "FM",
            category_name: "Farmaseutikal",
            data_param: d,
            sub_code: "FM",
            sub_name: "Farmaseutikal",
        },
        SubStrategy {
            category_code: "KO",
            category_name: "Kosmetik & Dandanan",
            data_param: d,
            sub_code: "KO",
            sub_name: "Kosmetik",
        },
        SubStrategy {
            category_code: "MD",
            category_name: "Peranti Perubatan",
            data_param: d,
            sub_code: "MD",
            sub_name: "Peranti Perubatan",
        },
        SubStrategy {
            category_code: "OEM",
            category_name: "OEM",
            data_param: d,
            sub_code: "OEM",
            sub_name: "OEM",
        },
        SubStrategy {
            category_code: "PR",
            category_name: "Produk Makanan/Minuman",
            data_param: d,
            sub_code: "PR",
            sub_name: "Produk",
        },
        // Premises / other
        SubStrategy {
            category_code: "PE",
            category_name: "Premis Makanan",
            data_param: d,
            sub_code: "HO",
            sub_name: "Hotel & Resort",
        },
        SubStrategy {
            category_code: "PE",
            category_name: "Premis Makanan",
            data_param: d,
            sub_code: "PE",
            sub_name: "Premis Makanan",
        },
        SubStrategy {
            category_code: "PS",
            category_name: "Rumah Sembelihan",
            data_param: d,
            sub_code: "RS",
            sub_name: "Rumah Sembelih",
        },
    ]
}

pub fn semaphore() -> Arc<Semaphore> {
    Arc::new(Semaphore::new(MAX_CONCURRENT))
}
