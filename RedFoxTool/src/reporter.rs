//! نظام التقارير
//! يولد تقارير بتنسيقات مختلفة

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use chrono::{Local, DateTime};
use serde_json::json;
use anyhow::{Result, Context};
use tokio::fs as tokio_fs;

use crate::scanner::ScanResult;

/// مولد التقارير
pub struct ReportGenerator {
    output_dir: PathBuf,
}

impl ReportGenerator {
    /// إنشاء مولد تقارير جديد
    pub fn new() -> Self {
        let output_dir = if cfg!(debug_assertions) {
            PathBuf::from("./reports")
        } else {
            PathBuf::from("/var/log/redfox/reports")
        };
        
        // إنشاء المجلد إذا لم يكن موجودًا
        std::fs::create_dir_all(&output_dir).ok();
        
        Self { output_dir }
    }
    
    /// توليد تقرير
    pub async fn generate(
        &self,
        results: &[ScanResult],
        base_filename: &str,
        format: &str,
    ) -> Result<String> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.{}", base_filename, timestamp, format);
        let filepath = self.output_dir.join(&filename);
        
        match format.to_lowercase().as_str() {
            "json" => self.generate_json(results, &filepath).await,
            "html" => self.generate_html(results, &filepath).await,
            "csv" => self.generate_csv(results, &filepath).await,
            "txt" => self.generate_text(results, &filepath).await,
            "xml" => self.generate_xml(results, &filepath).await,
            _ => {
                // الافتراضي: JSON
                self.generate_json(results, &filepath).await
            }
        }?;
        
        Ok(filepath.to_string_lossy().to_string())
    }
    
    /// توليد تقرير JSON
    async fn generate_json(&self, results: &[ScanResult], filepath: &Path) -> Result<()> {
        let successful: Vec<_> = results.iter().filter(|r| r.success).collect();
        let failed: Vec<_> = results.iter().filter(|r| !r.success).collect();
        
        let report = json!({
            "metadata": {
                "generated_at": chrono::Utc::now().to_rfc3339(),
                "total_results": results.len(),
                "successful_count": successful.len(),
                "failed_count": failed.len(),
                "success_rate": if results.is_empty() {
                    0.0
                } else {
                    (successful.len() as f64 / results.len() as f64) * 100.0
                }
            },
            "successful": successful.iter().map(|r| {
                json!({
                    "username": r.username,
                    "password": r.password,
                    "status_code": r.status_code,
                    "response_time_ms": r.response_time.as_millis(),
                    "timestamp": r.timestamp.to_rfc3339()
                })
            }).collect::<Vec<_>>(),
            "failed": failed.iter().take(100).map(|r| { // Limit failed to 100
                json!({
                    "username": r.username,
                    "password": r.password,
                    "error": r.error,
                    "timestamp": r.timestamp.to_rfc3339()
                })
            }).collect::<Vec<_>>(),
            "statistics": {
                "total_attempts": results.len(),
                "unique_users": {
                    let mut users: Vec<_> = results.iter().map(|r| &r.username).collect();
                    users.sort();
                    users.dedup();
                    users.len()
                },
                "unique_passwords": {
                    let mut passwords: Vec<_> = results.iter().map(|r| &r.password).collect();
                    passwords.sort();
                    passwords.dedup();
                    passwords.len()
                },
                "average_response_time_ms": {
                    if !results.is_empty() {
                        let total: u128 = results.iter()
                            .map(|r| r.response_time.as_millis())
                            .sum();
                        total / results.len() as u128
                    } else {
                        0
                    }
                }
            }
        });
        
        let json_string = serde_json::to_string_pretty(&report)?;
        tokio_fs::write(filepath, json_string).await?;
        
        Ok(())
    }
    
    /// توليد تقرير HTML
    async fn generate_html(&self, results: &[ScanResult], filepath: &Path) -> Result<()> {
        let successful: Vec<_> = results.iter().filter(|r| r.success).collect();
        let failed: Vec<_> = results.iter().filter(|r| !r.success).take(50).collect(); // Limit failed
        
        let success_rate = if results.is_empty() {
            0.0
        } else {
            (successful.len() as f64 / results.len() as f64) * 100.0
        };
        
        let html = format!(r#"
<!DOCTYPE html>
<html lang="ar" dir="rtl">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>تقرير RedFoxTool</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
        }}
        
        body {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 20px;
            color: #333;
        }}
        
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            border-radius: 20px;
            box-shadow: 0 20px 60px rgba(0,0,0,0.3);
            overflow: hidden;
        }}
        
        .header {{
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
            color: white;
            padding: 40px;
            text-align: center;
            position: relative;
            overflow: hidden;
        }}
        
        .header::before {{
            content: '';
            position: absolute;
            top: -50%;
            left: -50%;
            width: 200%;
            height: 200%;
            background: radial-gradient(circle, rgba(255,255,255,0.1) 1px, transparent 1px);
            background-size: 30px 30px;
            animation: move 20s linear infinite;
        }}
        
        @keyframes move {{
            0% {{ transform: rotate(0deg); }}
            100% {{ transform: rotate(360deg); }}
        }}
        
        .header h1 {{
            font-size: 3em;
            margin-bottom: 10px;
            position: relative;
            z-index: 1;
        }}
        
        .header .subtitle {{
            font-size: 1.2em;
            opacity: 0.9;
            position: relative;
            z-index: 1;
        }}
        
        .stats {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            padding: 30px;
            background: #f8f9fa;
        }}
        
        .stat-card {{
            background: white;
            padding: 25px;
            border-radius: 15px;
            box-shadow: 0 5px 15px rgba(0,0,0,0.1);
            text-align: center;
            transition: transform 0.3s;
        }}
        
        .stat-card:hover {{
            transform: translateY(-5px);
        }}
        
        .stat-card.success {{
            border-top: 5px solid #28a745;
        }}
        
        .stat-card.warning {{
            border-top: 5px solid #ffc107;
        }}
        
        .stat-card.danger {{
            border-top: 5px solid #dc3545;
        }}
        
        .stat-card.info {{
            border-top: 5px solid #17a2b8;
        }}
        
        .stat-value {{
            font-size: 2.5em;
            font-weight: bold;
            margin: 10px 0;
        }}
        
        .success .stat-value {{ color: #28a745; }}
        .warning .stat-value {{ color: #ffc107; }}
        .danger .stat-value {{ color: #dc3545; }}
        .info .stat-value {{ color: #17a2b8; }}
        
        .results {{
            padding: 30px;
        }}
        
        .section-title {{
            font-size: 1.8em;
            margin-bottom: 20px;
            color: #1a1a2e;
            border-bottom: 3px solid #667eea;
            padding-bottom: 10px;
        }}
        
        table {{
            width: 100%;
            border-collapse: collapse;
            margin-bottom: 30px;
            border-radius: 10px;
            overflow: hidden;
            box-shadow: 0 5px 15px rgba(0,0,0,0.1);
        }}
        
        th {{
            background: #1a1a2e;
            color: white;
            padding: 15px;
            text-align: right;
        }}
        
        td {{
            padding: 12px 15px;
            border-bottom: 1px solid #eee;
        }}
        
        tr:nth-child(even) {{
            background: #f8f9fa;
        }}
        
        tr:hover {{
            background: #e9ecef;
        }}
        
        .success-row {{
            background: #d4edda !important;
        }}
        
        .success-row:hover {{
            background: #c3e6cb !important;
        }}
        
        .footer {{
            background: #1a1a2e;
            color: white;
            padding: 20px;
            text-align: center;
            margin-top: 30px;
        }}
        
        .timestamp {{
            font-size: 0.9em;
            opacity: 0.8;
        }}
        
        @media (max-width: 768px) {{
            .header h1 {{ font-size: 2em; }}
            .stats {{ grid-template-columns: 1fr; }}
            table {{ display: block; overflow-x: auto; }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🦊 RedFoxTool Report</h1>
            <div class="subtitle">تقرير فحص المصادقة | {}</div>
        </div>
        
        <div class="stats">
            <div class="stat-card success">
                <div class="stat-label">المحاولات الناجحة</div>
                <div class="stat-value">{}</div>
                <div class="stat-desc">من إجمالي {} محاولة</div>
            </div>
            
            <div class="stat-card info">
                <div class="stat-label">معدل النجاح</div>
                <div class="stat-value">{:.1}%</div>
                <div class="stat-desc">نسبة النجاح الإجمالية</div>
            </div>
            
            <div class="stat-card warning">
                <div class="stat-label">المستخدمين الفريدين</div>
                <div class="stat-value">{}</div>
                <div class="stat-desc">عدد المستخدمين المختبرين</div>
            </div>
            
            <div class="stat-card danger">
                <div class="stat-label">كلمات المرور الفريدة</div>
                <div class="stat-value">{}</div>
                <div class="stat-desc">عدد كلمات المرور المختبرة</div>
            </div>
        </div>
        
        <div class="results">
            <h2 class="section-title">📊 النتائج الناجحة</h2>
            {}
            
            <h2 class="section-title">⚠️ المحاولات الفاشلة (عرض 50)</h2>
            {}
        </div>
        
        <div class="footer">
            <div class="timestamp">
                تم إنشاء التقرير في: {} |
                بواسطة RedFoxTool v1.0
            </div>
        </div>
    </div>
</body>
</html>
"#,
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            successful.len(),
            results.len(),
            success_rate,
            {
                let mut users: Vec<_> = results.iter().map(|r| &r.username).collect();
                users.sort();
                users.dedup();
                users.len()
            },
            {
                let mut passwords: Vec<_> = results.iter().map(|r| &r.password).collect();
                passwords.sort();
                passwords.dedup();
                passwords.len()
            },
            self.generate_successful_table(successful),
            self.generate_failed_table(failed),
            Local::now().format("%Y-%m-%d %H:%M:%S")
        );
        
        tokio_fs::write(filepath, html).await?;
        Ok(())
    }
    
    /// إنشاء جدول النتائج الناجحة
    fn generate_successful_table(&self, results: Vec<&ScanResult>) -> String {
        if results.is_empty() {
            return "<p style='text-align: center; padding: 20px; color: #666;'>لا توجد نتائج ناجحة</p>".to_string();
        }
        
        let mut table = String::from("<table>\n");
        table.push_str("<tr>\n");
        table.push_str("    <th>#</th>\n");
        table.push_str("    <th>اسم المستخدم</th>\n");
        table.push_str("    <th>كلمة المرور</th>\n");
        table.push_str("    <th>رمز الحالة</th>\n");
        table.push_str("    <th>وقت الاستجابة</th>\n");
        table.push_str("    <th>الوقت</th>\n");
        table.push_str("</tr>\n");
        
        for (i, result) in results.iter().enumerate() {
            let row_class = if i % 2 == 0 { "success-row" } else { "" };
            table.push_str(&format!(
                "<tr class='{}'>\n",
                row_class
            ));
            table.push_str(&format!("    <td>{}</td>\n", i + 1));
            table.push_str(&format!("    <td><strong>{}</strong></td>\n", result.username));
            table.push_str(&format!("    <td><code>{}</code></td>\n", result.password));
            table.push_str(&format!("    <td>{}</td>\n", result.status_code));
            table.push_str(&format!("    <td>{:.2?}</td>\n", result.response_time));
            table.push_str(&format!("    <td>{}</td>\n", 
                result.timestamp.with_timezone(&Local).format("%H:%M:%S")));
            table.push_str("</tr>\n");
        }
        
        table.push_str("</table>");
        table
    }
    
    /// إنشاء جدول المحاولات الفاشلة
    fn generate_failed_table(&self, results: Vec<&ScanResult>) -> String {
        if results.is_empty() {
            return "<p style='text-align: center; padding: 20px; color: #666;'>لا توجد محاولات فاشلة</p>".to_string();
        }
        
        let mut table = String::from("<table>\n");
        table.push_str("<tr>\n");
        table.push_str("    <th>اسم المستخدم</th>\n");
        table.push_str("    <th>كلمة المرور</th>\n");
        table.push_str("    <th>الخطأ</th>\n");
        table.push_str("</tr>\n");
        
        for result in results {
            table.push_str("<tr>\n");
            table.push_str(&format!("    <td>{}</td>\n", result.username));
            table.push_str(&format!("    <td>{}</td>\n", result.password));
            table.push_str(&format!("    <td>{}</td>\n", 
                result.error.as_deref().unwrap_or("غير معروف")));
            table.push_str("</tr>\n");
        }
        
        table.push_str("</table>");
        table
    }
    
    /// توليد تقرير CSV
    async fn generate_csv(&self, results: &[ScanResult], filepath: &Path) -> Result<()> {
        let mut csv_writer = csv::Writer::from_path(filepath)?;
        
        // كتابة العناوين
        csv_writer.write_record(&[
            "Username",
            "Password",
            "Success",
            "Status Code",
            "Response Time (ms)",
            "Error",
            "Timestamp"
        ])?;
        
        // كتابة البيانات
        for result in results {
            csv_writer.write_record(&[
                &result.username,
                &result.password,
                &result.success.to_string(),
                &result.status_code.to_string(),
                &result.response_time.as_millis().to_string(),
                result.error.as_deref().unwrap_or(""),
                &result.timestamp.to_rfc3339()
            ])?;
        }
        
        csv_writer.flush()?;
        Ok(())
    }
    
    /// توليد تقرير نصي
    async fn generate_text(&self, results: &[ScanResult], filepath: &Path) -> Result<()> {
        let mut text = String::new();
        let successful: Vec<_> = results.iter().filter(|r| r.success).collect();
        let failed_count = results.len() - successful.len();
        
        // الرأس
        text.push_str(&format!("{}\n", "=".repeat(70)));
        text.push_str("               تقرير RedFoxTool - نتائج فحص المصادقة\n");
        text.push_str(&format!("{}\n\n", "=".repeat(70)));
        
        // المعلومات الأساسية
        text.push_str(&format!("تاريخ التقرير: {}\n", Local::now().format("%Y-%m-%d %H:%M:%S")));
        text.push_str(&format!("إجمالي المحاولات: {}\n", results.len()));
        text.push_str(&format!("المحاولات الناجحة: {}\n", successful.len()));
        text.push_str(&format!("المحاولات الفاشلة: {}\n", failed_count));
        text.push_str(&format!("معدل النجاح: {:.1}%\n\n", 
            if results.is_empty() { 0.0 } else { (successful.len() as f64 / results.len() as f64) * 100.0 }));
        
        // النتائج الناجحة
        if !successful.is_empty() {
            text.push_str(&format!("{}\n", "-".repeat(70)));
            text.push_str("النتائج الناجحة:\n");
            text.push_str(&format!("{}\n", "-".repeat(70)));
            
            for (i, result) in successful.iter().enumerate() {
                text.push_str(&format!("{:3}. {:20} {:30} [{}] {:.2?}\n",
                    i + 1,
                    result.username,
                    result.password,
                    result.status_code,
                    result.response_time
                ));
            }
            text.push_str("\n");
        }
        
        // إحصائيات
        text.push_str(&format!("{}\n", "-".repeat(70)));
        text.push_str("الإحصائيات:\n");
        text.push_str(&format!("{}\n", "-".repeat(70)));
        
        let unique_users = {
            let mut users: Vec<_> = results.iter().map(|r| &r.username).collect();
            users.sort();
            users.dedup();
            users.len()
        };
        
        let unique_passwords = {
            let mut passwords: Vec<_> = results.iter().map(|r| &r.password).collect();
            passwords.sort();
            passwords.dedup();
            passwords.len()
        };
        
        let avg_response_time = if !results.is_empty() {
            let total: u128 = results.iter()
                .map(|r| r.response_time.as_millis())
                .sum();
            total / results.len() as u128
        } else {
            0
        };
        
        text.push_str(&format!("المستخدمين الفريدين: {}\n", unique_users));
        text.push_str(&format!("كلمات المرور الفريدة: {}\n", unique_passwords));
        text.push_str(&format!("متوسط وقت الاستجابة: {} مللي ثانية\n", avg_response_time));
        
        // الحواشي
        text.push_str(&format!("\n{}\n", "-".repeat(70)));
        text.push_str("ملاحظات:\n");
        text.push_str("• تم إنشاء هذا التقرير بواسطة RedFoxTool v1.0\n");
        text.push_str("• الاستخدام المسموح به فقط للأغراض القانونية\n");
        text.push_str(&format!("{}\n", "=".repeat(70)));
        
        tokio_fs::write(filepath, text).await?;
        Ok(())
    }
    
    /// توليد تقرير XML
    async fn generate_xml(&self, results: &[ScanResult], filepath: &Path) -> Result<()> {
        let successful: Vec<_> = results.iter().filter(|r| r.success).collect();
        let failed: Vec<_> = results.iter().filter(|r| !r.success).collect();
        
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<!DOCTYPE redfox-report SYSTEM \"https://redfox.security/dtd/report.dtd\">\n");
        xml.push_str("<redfox-report>\n");
        
        // المعلومات الوصفية
        xml.push_str("  <metadata>\n");
        xml.push_str(&format!("    <generated-at>{}</generated-at>\n", chrono::Utc::now().to_rfc3339()));
        xml.push_str(&format!("    <tool>RedFoxTool</tool>\n"));
        xml.push_str(&format!("    <version>1.0.0</version>\n"));
        xml.push_str(&format!("    <total-attempts>{}</total-attempts>\n", results.len()));
        xml.push_str(&format!("    <successful>{}</successful>\n", successful.len()));
        xml.push_str(&format!("    <failed>{}</failed>\n", failed.len()));
        xml.push_str(&format!("    <success-rate>{:.2}</success-rate>\n",
            if results.is_empty() { 0.0 } else { (successful.len() as f64 / results.len() as f64) * 100.0 }));
        xml.push_str("  </metadata>\n");
        
        // النتائج الناجحة
        if !successful.is_empty() {
            xml.push_str("  <successful-results>\n");
            for result in successful {
                xml.push_str("    <credential>\n");
                xml.push_str(&format!("      <username>{}</username>\n", escape_xml(&result.username)));
                xml.push_str(&format!("      <password>{}</password>\n", escape_xml(&result.password)));
                xml.push_str(&format!("      <status-code>{}</status-code>\n", result.status_code));
                xml.push_str(&format!("      <response-time-ms>{}</response-time-ms>\n", result.response_time.as_millis()));
                xml.push_str(&format!("      <timestamp>{}</timestamp>\n", result.timestamp.to_rfc3339()));
                xml.push_str("    </credential>\n");
            }
            xml.push_str("  </successful-results>\n");
        }
        
        // النتائج الفاشلة (محدودة)
        if !failed.is_empty() {
            xml.push_str("  <failed-results>\n");
            for result in failed.iter().take(100) {
                xml.push_str("    <attempt>\n");
                xml.push_str(&format!("      <username>{}</username>\n", escape_xml(&result.username)));
                xml.push_str(&format!("      <password>{}</password>\n", escape_xml(&result.password)));
                xml.push_str(&format!("      <error>{}</error>\n", 
                    escape_xml(result.error.as_deref().unwrap_or("unknown"))));
                xml.push_str(&format!("      <timestamp>{}</timestamp>\n", result.timestamp.to_rfc3339()));
                xml.push_str("    </attempt>\n");
            }
            xml.push_str("  </failed-results>\n");
        }
        
        xml.push_str("</redfox-report>");
        
        tokio_fs::write(filepath, xml).await?;
        Ok(())
    }
}

/// تهريب أحرف XML
fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

impl Default for ReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}