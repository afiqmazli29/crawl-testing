use std::sync::Arc;
use tokio::sync::Semaphore;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Category<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str)]);

pub const MAX_CONCURRENT: usize = 5;

pub fn categories() -> &'static [Category<'static>] {
    &[
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
    ]
}

pub fn semaphore() -> Arc<Semaphore> {
    Arc::new(Semaphore::new(MAX_CONCURRENT))
}
