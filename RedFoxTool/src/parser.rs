//! محلل المدخلات
//! يحلل ملفات ومتغيرات الإدخال

use std::fs;
use std::path::Path;
use tokio::fs as tokio_fs;
use anyhow::{Result, Context};
use glob::glob;

/// تحليل الإدخال (ملف أو نص)
pub async fn parse_input(input: &str) -> Result<Vec<String>> {
    // إذا كان الإدخال مسار ملف
    if Path::new(input).exists() {
        parse_file(input).await
    } else if input.contains(',') {
        // إذا كان نصًا مفصولًا بفواصل
        Ok(parse_comma_separated(input))
    } else if input.contains('\n') {
        // إذا كان نصًا متعدد الأسطر
        Ok(parse_multiline(input))
    } else {
        // إذا كان قيمة واحدة
        Ok(vec![input.to_string()])
    }
}

/// تحليل ملف
async fn parse_file(filepath: &str) -> Result<Vec<String>> {
    // التحقق من وجود الملف
    if !Path::new(filepath).exists() {
        // البحث في المسارات الشائعة
        let common_paths = [
            filepath,
            &format!("/usr/share/wordlists/{}", filepath),
            &format!("/usr/share/seclists/{}", filepath),
            &format!("/usr/share/redfox/wordlists/{}", filepath),
            &format!("~/.redfox/wordlists/{}", filepath),
        ];
        
        for path in &common_paths {
            let expanded = shellexpand::full(path)
                .context("فشل في توسيع المسار")?;
            
            if Path::new(&*expanded).exists() {
                return parse_file_contents(&expanded).await;
            }
        }
        
        return Err(anyhow::anyhow!("الملف غير موجود: {}", filepath));
    }
    
    parse_file_contents(filepath).await
}

/// تحليل محتويات الملف
async fn parse_file_contents(filepath: &str) -> Result<Vec<String>> {
    let content = tokio_fs::read_to_string(filepath)
        .await
        .context(format!("فشل في قراءة الملف: {}", filepath))?;
    
    let items: Vec<String> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect();
    
    if items.is_empty() {
        return Err(anyhow::anyhow!("الملف فارغ: {}", filepath));
    }
    
    Ok(items)
}

/// تحليل نص مفصول بفواصل
fn parse_comma_separated(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// تحليل نص متعدد الأسطر
fn parse_multiline(input: &str) -> Vec<String> {
    input
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect()
}

/// تحليل نمط جلوب للملفات
pub async fn parse_glob_pattern(pattern: &str) -> Result<Vec<String>> {
    let mut files = Vec::new();
    
    for entry in glob(pattern).context("نمط جلوب غير صالح")? {
        match entry {
            Ok(path) => {
                if path.is_file() {
                    files.push(path.to_string_lossy().to_string());
                }
            }
            Err(e) => log::warn!("خطأ في نمط جلوب: {}", e),
        }
    }
    
    if files.is_empty() {
        return Err(anyhow::anyhow!("لم يتم العثور على ملفات تطابق النمط: {}", pattern));
    }
    
    Ok(files)
}

/// دمج عدة مصادر
pub async fn merge_sources(sources: &[String]) -> Result<Vec<String>> {
    let mut all_items = Vec::new();
    
    for source in sources {
        let items = parse_input(source).await?;
        all_items.extend(items);
    }
    
    // إزالة التكرارات مع الحفاظ على الترتيب
    all_items.sort();
    all_items.dedup();
    
    Ok(all_items)
}

/// تحليل الإدخال مع توسيع الأنماط
pub async fn parse_input_with_expansion(input: &str) -> Result<Vec<String>> {
    // التحقق من الأنماط الخاصة
    if input.contains('*') || input.contains('?') || input.contains('[') {
        // نمط جلوب
        parse_glob_pattern(input).await
    } else if input.starts_with("file://") {
        // مسار ملف
        let filepath = input.trim_start_matches("file://");
        parse_file(filepath).await
    } else if input.starts_with("http://") || input.starts_with("https://") {
        // رابط URL (غير مدعوم حالياً)
        Err(anyhow::anyhow!("روابط URL غير مدعومة حالياً"))
    } else {
        // تحليل عادي
        parse_input(input).await
    }
}

/// تحويل المتجه إلى سلسلة مفصولة بفواصل
pub fn vec_to_comma_separated(items: &[String]) -> String {
    items.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_parse_comma_separated() {
        let input = "admin, user, test,guest";
        let result = parse_comma_separated(input);
        
        assert_eq!(result, vec!["admin", "user", "test", "guest"]);
    }
    
    #[tokio::test]
    async fn test_parse_multiline() {
        let input = "admin\nuser\ntest\n\nguest";
        let result = parse_multiline(input);
        
        assert_eq!(result, vec!["admin", "user", "test", "guest"]);
    }
    
    #[tokio::test]
    async fn test_parse_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "admin").unwrap();
        writeln!(temp_file, "user").unwrap();
        writeln!(temp_file, "# تعليق").unwrap();
        writeln!(temp_file, "test").unwrap();
        
        let result = parse_file(temp_file.path().to_str().unwrap()).await.unwrap();
        
        assert_eq!(result, vec!["admin", "user", "test"]);
    }
    
    #[tokio::test]
    async fn test_parse_input_single() {
        let input = "admin";
        let result = parse_input(input).await.unwrap();
        
        assert_eq!(result, vec!["admin"]);
    }
}