//! RedFoxTool - أداة تخمين كلمات مرور فائقة السرعة
//! مكتوبة بلغة Rust للأداء الأمثل
//! الإصدار: 1.0.0

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

use std::process;
use std::time::Instant;
use colored::Colorize;
use anyhow::{Result, Context};
use tokio::runtime::Runtime;

// استيراد الموديولات
mod cli;
mod scanner;
mod bruteforcer;
mod http_client;
mod parser;
mod validator;
mod progress;
mod reporter;
mod modules;
mod utils;

use cli::{Cli, Command};
use scanner::RedFoxScanner;
use reporter::ReportGenerator;
use utils::logger::Logger;

/// دالة رئيسية غير متزامنة
async fn async_main() -> Result<()> {
    // عرض البانر
    show_banner();
    
    // تحليل سطر الأوامر
    let cli = Cli::parse();
    
    // تهيئة المسجل
    let logger = Logger::new(cli.verbose);
    logger.info("بدء RedFoxTool");
    
    // التحقق من المتطلبات
    if cli.requires_root && !utils::system::is_root() {
        logger.error("يجب تشغيل الأداة كـ root!");
        process::exit(1);
    }
    
    match cli.command {
        Command::Scan {
            url,
            user,
            password_file,
            threads,
            timeout,
            output,
            format,
            verbose,
            proxy,
            mode,
            rate_limit,
            ..
        } => {
            let start_time = Instant::now();
            
            logger.info(&format!("بدء الفحص على: {}", url));
            logger.info(&format!("المستخدمون: {}", user));
            logger.info(&format!("خيوط المعالجة: {}", threads));
            
            // إنشاء الماسح
            let scanner = RedFoxScanner::new(
                &url,
                &user,
                &password_file,
                threads,
                timeout,
                mode,
                rate_limit,
            )
            .await
            .context("فشل في تهيئة الماسح")?;
            
            // تعيين البروكسي إذا وجد
            if let Some(proxy_url) = proxy {
                scanner.set_proxy(&proxy_url).await?;
            }
            
            // تشغيل الفحص
            let results = scanner
                .scan(verbose)
                .await
                .context("فشل في تنفيذ الفحص")?;
            
            // حساب الوقت المستغرق
            let duration = start_time.elapsed();
            
            // عرض النتائج
            display_results(&results, verbose, &logger);
            
            // إظهار الإحصائيات
            show_statistics(&results, duration, &logger);
            
            // حفظ النتائج
            if let Some(output_path) = output {
                save_results(&results, &output_path, format, &logger).await?;
            }
        }
        
        Command::Benchmark {
            url,
            users_file,
            passwords_file,
            iterations,
            threads,
        } => {
            logger.info("بدء اختبار الأداء");
            
            // تنفيذ اختبار الأداء
            modules::benchmark::run(
                &url,
                &users_file,
                &passwords_file,
                iterations,
                threads,
            )
            .await
            .context("فشل في اختبار الأداء")?;
        }
        
        Command::Generate {
            wordlist,
            size,
            patterns,
        } => {
            logger.info("توليد قائمة كلمات");
            
            modules::generator::generate(
                &wordlist,
                size,
                patterns.as_deref(),
            )
            .await
            .context("فشل في توليد القائمة")?;
        }
        
        Command::Validate { url } => {
            logger.info("التحقق من الهدف");
            
            let is_valid = validator::validate_url(&url)
                .await
                .context("فشل في التحقق")?;
            
            if is_valid {
                logger.success("الهدف صالح للفحص");
            } else {
                logger.error("الهدف غير صالح");
            }
        }
        
        Command::ListWordlists => {
            logger.info("عرض قوائم الكلمات المتاحة");
            
            let wordlists = utils::wordlists::list_available();
            if wordlists.is_empty() {
                logger.warn("لا توجد قوائم كلمات متاحة");
            } else {
                for (i, wordlist) in wordlists.iter().enumerate() {
                    println!("{}. {}", i + 1, wordlist.green());
                }
            }
        }
        
        Command::Update => {
            logger.info("التحقق من التحديثات");
            
            utils::updater::check_for_updates()
                .await
                .context("فشل في التحقق من التحديثات")?;
        }
    }
    
    logger.info("اكتمل التنفيذ بنجاح");
    Ok(())
}

/// عرض البانر
fn show_banner() {
    let banner = r#"
    ██████╗ ███████╗██████╗ ███████╗ ██████╗ ██╗  ██╗
    ██╔══██╗██╔════╝██╔══██╗██╔════╝██╔═══██╗╚██╗██╔╝
    ██████╔╝█████╗  ██║  ██║█████╗  ██║   ██║ ╚███╔╝ 
    ██╔══██╗██╔══╝  ██║  ██║██╔══╝  ██║   ██║ ██╔██╗ 
    ██║  ██║███████╗██████╔╝██║     ╚██████╔╝██╔╝ ██╗
    ╚═╝  ╚═╝╚══════╝╚═════╝ ╚═╝      ╚═════╝ ╚═╝  ╚═╝
    
    RedFoxTool v1.0.0 - Ultra Fast Password Auditor
    ===============================================
    "#.bright_red();
    
    println!("{}", banner);
}

/// عرض النتائج
fn display_results(results: &[crate::scanner::ScanResult], verbose: bool, logger: &Logger) {
    if results.is_empty() {
        logger.warn("لم يتم العثور على نتائج");
        return;
    }
    
    let successes: Vec<_> = results.iter().filter(|r| r.success).collect();
    
    if !successes.is_empty() {
        println!("\n{}", "نتائج ناجحة:".bright_green().bold());
        println!("{}", "-".repeat(60).bright_blue());
        
        for (i, result) in successes.iter().enumerate() {
            println!(
                "{:3}. {:<20} {:<30} [{}] {:.2?}",
                i + 1,
                result.username.bright_cyan(),
                result.password.bright_yellow(),
                result.status_code,
                result.response_time
            );
        }
    }
    
    if verbose {
        let failures: Vec<_> = results.iter().filter(|r| !r.success).collect();
        if !failures.is_empty() {
            println!("\n{}", "محاولات فاشلة:".bright_yellow().bold());
            for result in failures.iter().take(10) {
                println!(
                    "✗ {:<20} {:<30} - {}",
                    result.username,
                    result.password,
                    result.error.as_deref().unwrap_or("غير معروف")
                );
            }
            
            if failures.len() > 10 {
                println!("... و {} محاولة أخرى", failures.len() - 10);
            }
        }
    }
}

/// عرض الإحصائيات
fn show_statistics(results: &[crate::scanner::ScanResult], duration: std::time::Duration, logger: &Logger) {
    let total = results.len();
    let successes = results.iter().filter(|r| r.success).count();
    let failures = total - successes;
    let rps = total as f64 / duration.as_secs_f64();
    
    println!("\n{}", "إحصائيات الفحص:".bright_magenta().bold());
    println!("{}", "=".repeat(60).bright_blue());
    println!("الوقت المستغرق:          {:.2?}", duration);
    println!("إجمالي المحاولات:       {}", total);
    println!("المحاولات الناجحة:      {}", successes.to_string().bright_green());
    println!("المحاولات الفاشلة:      {}", failures.to_string().bright_red());
    println!("معدل المحاولات/ثانية:  {:.2}", rps.to_string().bright_yellow());
    
    if successes > 0 {
        let success_rate = (successes as f64 / total as f64) * 100.0;
        println!("معدل النجاح:            {:.2}%", success_rate);
    }
}

/// حفظ النتائج
async fn save_results(
    results: &[crate::scanner::ScanResult],
    output_path: &str,
    format: Option<String>,
    logger: &Logger,
) -> Result<()> {
    let generator = ReportGenerator::new();
    let format = format.unwrap_or_else(|| "json".to_string());
    
    let report_path = generator
        .generate(results, output_path, &format)
        .await
        .context("فشل في إنشاء التقرير")?;
    
    logger.success(&format!("تم حفظ التقرير في: {}", report_path));
    Ok(())
}

/// نقطة الدخول الرئيسية
fn main() {
    // إنشاء وقت تشغيل Tokio
    let rt = Runtime::new().unwrap_or_else(|e| {
        eprintln!("فشل في إنشاء وقت التشغيل: {}", e);
        process::exit(1);
    });
    
    // تشغيل الدالة الرئيسية
    if let Err(e) = rt.block_on(async_main()) {
        eprintln!("{}: {}", "خطأ".bright_red(), e);
        
        // عرض التفاصيل في الوضع التفصيلي
        if std::env::var("RUST_BACKTRACE").is_ok() {
            eprintln!("\nتفاصيل الخطأ:");
            for cause in e.chain() {
                eprintln!("  - {}", cause);
            }
        }
        
        process::exit(1);
    }
}