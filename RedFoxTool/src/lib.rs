//! مكتبة RedFoxTool الأساسية
//! توفر واجهة برمجية لاستخدام الأداة كمكتبة

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod scanner;
pub mod bruteforcer;
pub mod http_client;
pub mod parser;
pub mod validator;
pub mod progress;
pub mod reporter;
pub mod modules;
pub mod utils;

// إعادة تصدير الأنواع الأساسية
pub use scanner::{RedFoxScanner, ScanResult, ScanOptions};
pub use bruteforcer::{Bruteforcer, AttackMode};
pub use http_client::HttpClient;
pub use validator::ValidationResult;

/// تهيئة الأداة
pub fn init() {
    // تهيئة المسجل
    utils::logger::init();
    
    // التحقق من المتطلبات
    utils::system::check_requirements();
}

/// تنفيذ فحص سريع
pub async fn quick_scan(
    url: &str,
    username: &str,
    passwords: &[&str],
) -> anyhow::Result<Vec<ScanResult>> {
    let scanner = RedFoxScanner::new(
        url,
        username,
        "",
        10,
        30,
        "normal",
        None,
    )
    .await?;
    
    let results = scanner.scan_specific_passwords(passwords).await?;
    Ok(results)
}

/// توليد تقرير
pub async fn generate_report(
    results: &[ScanResult],
    format: &str,
    output_path: &str,
) -> anyhow::Result<String> {
    let reporter = reporter::ReportGenerator::new();
    let path = reporter.generate(results, output_path, format).await?;
    Ok(path)
}

/// التحقق من صحة الهدف
pub async fn validate_target(url: &str) -> anyhow::Result<ValidationResult> {
    validator::validate_url(url).await
}

/// معلومات الإصدار
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// معلومات المؤلف
pub fn author() -> &'static str {
    env!("CARGO_PKG_AUTHORS")
}