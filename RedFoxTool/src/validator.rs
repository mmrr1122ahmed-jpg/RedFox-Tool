//! مدقق البيانات
//! يتحقق من صحة المدخلات والمخرجات

use std::net::IpAddr;
use std::str::FromStr;
use url::Url;
use regex::Regex;
use anyhow::{Result, Context};

/// نتيجة التحقق
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// إنشاء نتيجة جديدة
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    /// إضافة خطأ
    pub fn add_error(&mut self, error: String) {
        self.is_valid = false;
        self.errors.push(error);
    }
    
    /// إضافة تحذير
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    /// التحقق مما إذا كان هناك أخطاء
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    
    /// عرض النتيجة
    pub fn display(&self) {
        if !self.errors.is_empty() {
            println!("{}", "أخطاء:".bright_red());
            for error in &self.errors {
                println!("  • {}", error);
            }
        }
        
        if !self.warnings.is_empty() {
            println!("{}", "تحذيرات:".bright_yellow());
            for warning in &self.warnings {
                println!("  • {}", warning);
            }
        }
        
        if self.is_valid {
            println!("{}", "التحقق ناجح!".bright_green());
        }
    }
}

/// التحقق من صحة عنوان URL
pub async fn validate_url(url: &str) -> Result<ValidationResult> {
    let mut result = ValidationResult::new();
    
    // التحقق من وجود البروتوكول
    if !url.starts_with("http://") && !url.starts_with("https://") {
        result.add_error("يجب أن يبدأ الرابط بـ http:// أو https://".to_string());
        return Ok(result);
    }
    
    // التحقق من صيغة URL
    match Url::parse(url) {
        Ok(parsed_url) => {
            // التحقق من وجود النطاق
            if parsed_url.host().is_none() {
                result.add_error("رابط غير صالح: لا يوجد نطاق".to_string());
            }
            
            // التحقق من المنفذ إذا كان موجودًا
            if let Some(port) = parsed_url.port() {
                if port == 0 || port > 65535 {
                    result.add_error(format!("رقم المنفذ غير صالح: {}", port));
                }
            }
        }
        Err(e) => {
            result.add_error(format!("رابط غير صالح: {}", e));
        }
    }
    
    // تحذيرات
    if url.contains("localhost") || url.contains("127.0.0.1") {
        result.add_warning("الرابط يشير إلى مضيف محلي".to_string());
    }
    
    if url.contains(":80/") && url.starts_with("http://") {
        result.add_warning("المنفذ 80 هو الافتراضي لـ HTTP".to_string());
    }
    
    if url.contains(":443/") && url.starts_with("https://") {
        result.add_warning("المنفذ 443 هو الافتراضي لـ HTTPS".to_string());
    }
    
    Ok(result)
}

/// التحقق من صحة اسم الملف
pub fn validate_filename(filename: &str) -> ValidationResult {
    let mut result = ValidationResult::new();
    
    // قائمة الأحرف غير المسموحة
    let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    
    for ch in invalid_chars {
        if filename.contains(ch) {
            result.add_error(format!("اسم الملف يحتوي على حرف غير مسموح: '{}'", ch));
            break;
        }
    }
    
    // التحقق من طول اسم الملف
    if filename.len() > 255 {
        result.add_error("اسم الملف طويل جدًا (أقصى حد: 255 حرف)".to_string());
    }
    
    // التحقق من أن اسم الملف ليس فارغًا
    if filename.trim().is_empty() {
        result.add_error("اسم الملف لا يمكن أن يكون فارغًا".to_string());
    }
    
    // التحقق من الامتداد
    if !filename.contains('.') {
        result.add_warning("اسم الملف بدون امتداد".to_string());
    }
    
    result
}

/// التحقق من صحة عنوان IP
pub fn validate_ip(ip_str: &str) -> ValidationResult {
    let mut result = ValidationResult::new();
    
    match IpAddr::from_str(ip_str) {
        Ok(ip) => {
            // تحذيرات خاصة
            if ip.is_loopback() {
                result.add_warning("عنوان IP هو loopback (127.0.0.1)".to_string());
            }
            
            if ip.is_private() {
                result.add_warning("عنوان IP خاص (private)".to_string());
            }
            
            if ip.is_multicast() {
                result.add_warning("عنوان IP هو multicast".to_string());
            }
        }
        Err(_) => {
            result.add_error("عنوان IP غير صالح".to_string());
        }
    }
    
    result
}

/// التحقق من صحة البروكسي
pub fn validate_proxy(proxy_url: &str) -> ValidationResult {
    let mut result = ValidationResult::new();
    
    // الأنماط المسموحة
    let patterns = [
        ("http://", 80),
        ("https://", 443),
        ("socks4://", 1080),
        ("socks5://", 1080),
    ];
    
    let mut matched = false;
    for (prefix, default_port) in patterns {
        if proxy_url.starts_with(prefix) {
            matched = true;
            
            // استخراج الجزء بعد البروتوكول
            let rest = &proxy_url[prefix.len()..];
            
            // التحقق من وجود المنفذ
            if !rest.contains(':') {
                result.add_warning(format!("البروكسي بدون منفذ، سيستخدم المنفذ {}", default_port));
            } else {
                // التحقق من صحة المنفذ
                let parts: Vec<&str> = rest.split(':').collect();
                if parts.len() == 2 {
                    if let Ok(port) = parts[1].parse::<u16>() {
                        if port == 0 || port > 65535 {
                            result.add_error(format!("رقم المنفذ غير صالح: {}", port));
                        }
                    } else {
                        result.add_error("رقم المنفذ غير صالح".to_string());
                    }
                }
            }
            
            break;
        }
    }
    
    if !matched {
        result.add_error("صيغة البروكسي غير صالحة. استخدم: http://, https://, socks4://, socks5://".to_string());
    }
    
    result
}

/// التحقق من صحة ملف كلمات المرور
pub async fn validate_password_file(filepath: &str) -> Result<ValidationResult> {
    let mut result = ValidationResult::new();
    
    // التحقق من وجود الملف
    if !std::path::Path::new(filepath).exists() {
        result.add_error(format!("الملف غير موجود: {}", filepath));
        return Ok(result);
    }
    
    // التحقق من صلاحيات القراءة
    match std::fs::metadata(filepath) {
        Ok(metadata) => {
            use std::os::unix::fs::PermissionsExt;
            let permissions = metadata.permissions();
            
            if permissions.mode() & 0o400 == 0 {
                result.add_warning("صلاحيات القراءة للملف محدودة".to_string());
            }
        }
        Err(e) => {
            result.add_error(format!("فشل في قراءة معلومات الملف: {}", e));
        }
    }
    
    // التحقق من حجم الملف
    match std::fs::metadata(filepath) {
        Ok(metadata) => {
            let size = metadata.len();
            
            if size == 0 {
                result.add_error("الملف فارغ".to_string());
            } else if size > 100 * 1024 * 1024 { // 100MB
                result.add_warning("ملف كبير جدًا، قد يؤثر على الأداء".to_string());
            } else if size < 100 { // 100 bytes
                result.add_warning("ملف صغير جدًا، قد لا يكون فعالاً".to_string());
            }
        }
        Err(e) => {
            result.add_error(format!("فشل في قراءة حجم الملف: {}", e));
        }
    }
    
    // محاولة قراءة أول 10 سطور للتحقق
    match std::fs::read_to_string(filepath) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().take(10).collect();
            let non_empty_lines: Vec<&str> = lines
                .iter()
                .filter(|line| !line.trim().is_empty())
                .copied()
                .collect();
            
            if non_empty_lines.is_empty() {
                result.add_error("الملف لا يحتوي على بيانات صالحة".to_string());
            } else {
                // التحقق من طول كلمات المرور
                for line in non_empty_lines {
                    let trimmed = line.trim();
                    if trimmed.len() > 100 {
                        result.add_warning("كلمات المرور طويلة جدًا قد تؤثر على الأداء".to_string());
                        break;
                    }
                }
            }
        }
        Err(e) => {
            result.add_error(format!("فشل في قراءة الملف: {}", e));
        }
    }
    
    Ok(result)
}

/// التحقق من صحة عدد الخيوط
pub fn validate_threads(threads: usize) -> ValidationResult {
    let mut result = ValidationResult::new();
    
    let max_threads = num_cpus::get() * 4;
    
    if threads == 0 {
        result.add_error("عدد الخيوط لا يمكن أن يكون صفرًا".to_string());
    } else if threads > max_threads {
        result.add_warning(format!(
            "عدد الخيوط كبير جدًا ({}). الحد المقترح: {}",
            threads, num_cpus::get() * 2
        ));
    } else if threads < 2 {
        result.add_warning("عدد قليل من الخيوط قد يقلل الأداء".to_string());
    }
    
    result
}

/// التحقق من صحة المهلة
pub fn validate_timeout(timeout: u64) -> ValidationResult {
    let mut result = ValidationResult::new();
    
    if timeout == 0 {
        result.add_error("المهلة لا يمكن أن تكون صفرًا".to_string());
    } else if timeout > 300 {
        result.add_warning("مهلة طويلة جدًا (أقصى حد موصى به: 60 ثانية)".to_string());
    } else if timeout < 5 {
        result.add_warning("مهلة قصيرة جدًا قد تسبب فشل الطلبات".to_string());
    }
    
    result
}

/// التحقق من صحة الهدف الشامل
pub async fn validate_target(url: &str, threads: usize, timeout: u64) -> Result<ValidationResult> {
    let mut result = ValidationResult::new();
    
    // التحقق من URL
    let url_result = validate_url(url).await?;
    if !url_result.is_valid {
        for error in url_result.errors {
            result.add_error(error);
        }
    }
    for warning in url_result.warnings {
        result.add_warning(warning);
    }
    
    // التحقق من الخيوط
    let threads_result = validate_threads(threads);
    if !threads_result.is_valid {
        for error in threads_result.errors {
            result.add_error(error);
        }
    }
    for warning in threads_result.warnings {
        result.add_warning(warning);
    }
    
    // التحقق من المهلة
    let timeout_result = validate_timeout(timeout);
    if !timeout_result.is_valid {
        for error in timeout_result.errors {
            result.add_error(error);
        }
    }
    for warning in timeout_result.warnings {
        result.add_warning(warning);
    }
    
    Ok(result)
}