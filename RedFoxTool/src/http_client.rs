//! عميل HTTP سريع ومتعدد الخيوط
//! يدعم TLS، البروكسي، وإعادة المحاولة

use std::sync::Arc;
use std::time::{Instant, Duration};
use reqwest::{Client, ClientBuilder, Response, Proxy, StatusCode};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, CONTENT_TYPE, COOKIE};
use serde_json::Value;
use tokio::time::{sleep, timeout};
use anyhow::{Result, Context};
use once_cell::sync::Lazy;

static USER_AGENTS: Lazy<Vec<&str>> = Lazy::new(|| {
    vec![
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15",
        "RedFoxTool/1.0",
    ]
});

/// عميل HTTP متقدم
pub struct HttpClient {
    client: Client,
    base_url: String,
    default_headers: HeaderMap,
    request_timeout: Duration,
    max_retries: u32,
    cookies: Option<String>,
}

impl HttpClient {
    /// إنشاء عميل جديد
    pub async fn new(
        base_url: &str,
        timeout_secs: u64,
        proxy: Option<&str>,
    ) -> Result<Self> {
        let mut builder = ClientBuilder::new()
            .connect_timeout(Duration::from_secs(10))
            .tcp_nodelay(true)
            .use_rustls_tls()
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(Duration::from_secs(90))
            .http1_only()
            .http2_prior_knowledge();
        
        // إضافة بروكسي إذا وجد
        if let Some(proxy_url) = proxy {
            let proxy = Proxy::all(proxy_url)
                .context("فشل في إنشاء بروكسي")?;
            builder = builder.proxy(proxy);
        }
        
        // إنشاء العميل
        let client = builder
            .build()
            .context("فشل في بناء عميل HTTP")?;
        
        // إنشاء الترويسات الافتراضية
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(USER_AGENTS[0])
        );
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded")
        );
        headers.insert(
            "Accept",
            HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        );
        headers.insert(
            "Accept-Language",
            HeaderValue::from_static("en-US,en;q=0.9")
        );
        headers.insert(
            "Accept-Encoding",
            HeaderValue::from_static("gzip, deflate, br")
        );
        headers.insert(
            "Connection",
            HeaderValue::from_static("keep-alive")
        );
        headers.insert(
            "Upgrade-Insecure-Requests",
            HeaderValue::from_static("1")
        );
        
        Ok(Self {
            client,
            base_url: base_url.to_string(),
            default_headers: headers,
            request_timeout: Duration::from_secs(timeout_secs),
            max_retries: 3,
            cookies: None,
        })
    }
    
    /// تعيين الكوكيز
    pub fn set_cookies(&mut self, cookies: &str) {
        self.cookies = Some(cookies.to_string());
    }
    
    /// اختبار تسجيل الدخول مع إعادة المحاولة
    pub async fn test_login(&self, username: &str, password: &str) -> Result<Response> {
        let mut retries = 0;
        let mut last_error = None;
        
        while retries <= self.max_retries {
            let start = Instant::now();
            
            match self.send_login_request(username, password).await {
                Ok(response) => {
                    let elapsed = start.elapsed();
                    
                    // تسجيل وقت الاستجابة
                    if elapsed > Duration::from_secs(5) {
                        log::warn!("استجابة بطيئة: {:.2?} - {}:{}", elapsed, username, password);
                    }
                    
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    retries += 1;
                    
                    if retries > self.max_retries {
                        break;
                    }
                    
                    // انتظار قبل إعادة المحاولة
                    let delay = Duration::from_millis(200 * retries as u64);
                    sleep(delay).await;
                }
            }
        }
        
        Err(anyhow::anyhow!(
            "فشل بعد {} محاولات: {}",
            self.max_retries,
            last_error.unwrap()
        ))
    }
    
    /// إرسال طلب تسجيل الدخول
    async fn send_login_request(&self, username: &str, password: &str) -> Result<Response> {
        let mut headers = self.default_headers.clone();
        
        // إضافة الكوكيز إذا وجدت
        if let Some(cookies) = &self.cookies {
            headers.insert(
                COOKIE,
                HeaderValue::from_str(cookies)?
            );
        }
        
        // بيانات النموذج
        let form_data = [
            ("username", username),
            ("password", password),
            ("submit", "Login"),
            ("csrf_token", "test"), // يمكن تعديله حسب الحاجة
        ];
        
        // إرسال الطلب مع مهلة
        let response = timeout(
            self.request_timeout,
            self.client
                .post(&self.base_url)
                .headers(headers)
                .form(&form_data)
        )
        .await
        .context("مهلة الطلب انتهت")?
        .send()
        .await
        .context("فشل في إرسال الطلب")?;
        
        Ok(response)
    }
    
    /// اختبار سريع بدون تحميل كامل الاستجابة
    pub async fn quick_test(&self, username: &str, password: &str) -> Result<bool> {
        let response = self.test_login(username, password).await?;
        
        // التحقق السريع من النجاح
        let success = self.is_success_response(&response).await;
        
        Ok(success)
    }
    
    /// التحقق من نجاح الاستجابة
    async fn is_success_response(&self, response: &Response) -> bool {
        let status = response.status();
        
        // التحقق من الحالة مباشرة
        if status.is_success() {
            return true;
        }
        
        // في بعض الأنظمة، التحويل قد يعني النجاح
        if status.is_redirection() {
            if let Some(location) = response.headers().get("Location") {
                let location_str = location.to_str().unwrap_or("");
                return !location_str.contains("login") && 
                       !location_str.contains("error") &&
                       !location_str.contains("fail");
            }
        }
        
        // التحقق من محتوى الاستجابة
        match response.text().await {
            Ok(body) => {
                // مؤشرات الفشل
                let failure_indicators = [
                    "invalid", "incorrect", "wrong", "failed", "error",
                    "login failed", "access denied", "unauthorized",
                ];
                
                // مؤشرات النجاح
                let success_indicators = [
                    "welcome", "dashboard", "home", "logout", "profile",
                    "success", "logged in", "redirecting",
                ];
                
                let body_lower = body.to_lowercase();
                
                // حساب النقاط
                let failure_points: usize = failure_indicators
                    .iter()
                    .map(|indicator| body_lower.matches(indicator).count())
                    .sum();
                
                let success_points: usize = success_indicators
                    .iter()
                    .map(|indicator| body_lower.matches(indicator).count())
                    .sum();
                
                success_points > failure_points
            }
            Err(_) => false,
        }
    }
    
    /// إرسال طلبات متعددة بالتوازي
    pub async fn send_batch(
        &self,
        credentials: &[(String, String)],
        concurrency: usize,
    ) -> Result<Vec<(String, String, bool, u16)>> {
        use tokio::sync::Semaphore;
        
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut tasks = Vec::new();
        
        for (username, password) in credentials {
            let client = self.client.clone();
            let url = self.base_url.clone();
            let headers = self.default_headers.clone();
            let u = username.clone();
            let p = password.clone();
            let semaphore = Arc::clone(&semaphore);
            
            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                let form_data = [("username", &u), ("password", &p)];
                
                match client
                    .post(&url)
                    .headers(headers)
                    .form(&form_data)
                    .timeout(Duration::from_secs(30))
                    .send()
                    .await
                {
                    Ok(resp) => (u, p, resp.status().is_success(), resp.status().as_u16()),
                    Err(_) => (u, p, false, 0),
                }
            });
            
            tasks.push(task);
        }
        
        // جمع النتائج
        let mut results = Vec::new();
        for task in tasks {
            if let Ok(result) = task.await {
                results.push(result);
            }
        }
        
        Ok(results)
    }
    
    /// اختبار الاتصال بالهدف
    pub async fn test_connection(&self) -> Result<bool> {
        match timeout(
            Duration::from_secs(10),
            self.client.get(&self.base_url).send()
        )
        .await
        {
            Ok(Ok(response)) => Ok(response.status().is_success()),
            _ => Ok(false),
        }
    }
    
    /// الحصول على إحصائيات العميل
    pub fn get_stats(&self) -> Value {
        serde_json::json!({
            "base_url": self.base_url,
            "timeout_seconds": self.request_timeout.as_secs(),
            "max_retries": self.max_retries,
            "has_cookies": self.cookies.is_some(),
        })
    }
}

impl Clone for HttpClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            default_headers: self.default_headers.clone(),
            request_timeout: self.request_timeout,
            max_retries: self.max_retries,
            cookies: self.cookies.clone(),
        }
    }
}