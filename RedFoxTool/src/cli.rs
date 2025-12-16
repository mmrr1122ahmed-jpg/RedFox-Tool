//! معالج سطر الأوامر لـ RedFoxTool
//! يستخدم Clap لتحليل الوسائط

use clap::{Parser, Subcommand, ArgAction};
use std::path::PathBuf;

/// الوسائط الأساسية لـ RedFoxTool
#[derive(Parser, Debug)]
#[command(
    name = "RedFoxTool",
    author = "RedFox Security Team",
    version = "1.0.0",
    about = "أداة تخمين كلمات مرور فائقة السرعة",
    long_about = r#"
RedFoxTool - أداة اختبار اختراق متقدمة لأنظمة المصادقة

مميزات:
  • سرعة فائقة باستخدام Rust و Tokio
  • دعم متعدد الخيوط والموازاة
  • أنماط هجوم متنوعة (Fast, Normal, Stealth, Aggressive)
  • دعم البروكسي والـ VPN
  • تقارير بتنسيقات متعددة (JSON, HTML, CSV, TXT)
  • نظام تسجيل متقدم
  • اختبار أداء مدمج

أمثلة:
  redfox scan --url http://target.com/login -U admin -P passwords.txt
  redfox scan --url https://target.com -U users.txt -P rockyou.txt -T 50 --mode fast
  redfox benchmark --url http://test.com --users users.txt --passwords passwords.txt
    "#
)]
pub struct Cli {
    /// الأمر المطلوب تنفيذه
    #[command(subcommand)]
    pub command: Command,
    
    /// الوضع التفصيلي
    #[arg(short, long, global = true, action = ArgAction::Count)]
    pub verbose: u8,
    
    /// الوضع الهادئ (عدم عرض البانر)
    #[arg(short, long, global = true)]
    pub quiet: bool,
    
    /// التشغيل كـ root (مطلوب لبعض الميزات)
    #[arg(long, global = true)]
    pub requires_root: bool,
    
    /// ملف الإعدادات
    #[arg(short, long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,
}

/// الأوامر المتاحة
#[derive(Subcommand, Debug)]
pub enum Command {
    /// تنفيذ فحص على هدف
    #[command(arg_required_else_help = true)]
    Scan {
        /// رابط صفحة تسجيل الدخول (مطلوب)
        #[arg(short, long, value_name = "URL")]
        url: String,
        
        /// اسم المستخدم أو ملف المستخدمين
        #[arg(short, long, value_name = "USER|FILE")]
        user: String,
        
        /// ملف كلمات المرور (مطلوب)
        #[arg(short = 'P', long, value_name = "FILE")]
        password_file: String,
        
        /// عدد الخيوط المتوازية
        #[arg(short, long, default_value_t = 20, value_name = "NUM")]
        threads: usize,
        
        /// مهلة الطلب بالثواني
        #[arg(long, default_value_t = 30, value_name = "SECONDS")]
        timeout: u64,
        
        /// حفظ النتائج في ملف
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,
        
        /// تنسيق المخرجات [txt, json, html, csv, xml]
        #[arg(long, value_name = "FORMAT")]
        format: Option<String>,
        
        /// الوضع التفصيلي
        #[arg(short, long)]
        verbose: bool,
        
        /// خادم بروكسي (مثال: http://127.0.0.1:8080)
        #[arg(long, value_name = "URL")]
        proxy: Option<String>,
        
        /// وضع الهجوم [fast, normal, stealth, aggressive]
        #[arg(short, long, default_value = "normal", value_name = "MODE")]
        mode: String,
        
        /// تحديد حد المعدل (طلبات/ثانية)
        #[arg(long, value_name = "RPS")]
        rate_limit: Option<u32>,
        
        /// حقل اسم المستخدم في النموذج
        #[arg(long, default_value = "username", value_name = "FIELD")]
        username_field: String,
        
        /// حقل كلمة المرور في النموذج
        #[arg(long, default_value = "password", value_name = "FIELD")]
        password_field: String,
        
        /// ملف الكوكيز (للمصادقة)
        #[arg(long, value_name = "FILE")]
        cookies: Option<String>,
        
        /// ترويسات HTTP مخصصة
        #[arg(long, value_name = "JSON")]
        headers: Option<String>,
        
        /// بيانات POST إضافية
        #[arg(long, value_name = "JSON")]
        data: Option<String>,
    },
    
    /// اختبار أداء الأداة
    #[command(arg_required_else_help = true)]
    Benchmark {
        /// رابط الهدف للاختبار
        #[arg(short, long, value_name = "URL")]
        url: String,
        
        /// ملف المستخدمين للاختبار
        #[arg(long, value_name = "FILE")]
        users_file: String,
        
        /// ملف كلمات المرور للاختبار
        #[arg(long, value_name = "FILE")]
        passwords_file: String,
        
        /// عدد مرات التكرار
        #[arg(short, long, default_value_t = 3, value_name = "NUM")]
        iterations: u32,
        
        /// عدد الخيوط
        #[arg(short, long, default_value_t = num_cpus::get(), value_name = "NUM")]
        threads: usize,
    },
    
    /// توليد قائمة كلمات مخصصة
    #[command(arg_required_else_help = true)]
    Generate {
        /// اسم ملف الإخراج
        #[arg(short, long, value_name = "FILE")]
        wordlist: String,
        
        /// حجم القائمة
        #[arg(short, long, default_value_t = 10000, value_name = "NUM")]
        size: usize,
        
        /// أنماط التوليد
        #[arg(short, long, value_name = "PATTERNS")]
        patterns: Option<Vec<String>>,
    },
    
    /// التحقق من صحة الهدف
    Validate {
        /// رابط الهدف للتحقق
        #[arg(value_name = "URL")]
        url: String,
    },
    
    /// عرض قوائم الكلمات المتاحة
    ListWordlists,
    
    /// التحقق من التحديثات
    Update,
}

impl Cli {
    /// تحليل سطر الأوامر
    pub fn parse() -> Self {
        let cli = Self::parse_from(std::env::args());
        
        // عرض البانر إذا لم يكن الوضع هادئًا
        if !cli.quiet {
            // سيتم عرض البانر في main.rs
        }
        
        cli
    }
    
    /// التحقق مما إذا كان الأمر يحتاج للجذر
    pub fn requires_root(&self) -> bool {
        self.requires_root
    }
    
    /// الحصول على مستوى التفاصيل
    pub fn verbosity(&self) -> u8 {
        self.verbose
    }
}

/// إعدادات الفحص
#[derive(Debug, Clone)]
pub struct ScanSettings {
    pub url: String,
    pub user_input: String,
    pub password_file: String,
    pub threads: usize,
    pub timeout: u64,
    pub mode: AttackMode,
    pub rate_limit: Option<u32>,
    pub proxy: Option<String>,
    pub output_format: String,
}

/// أنماط الهجوم
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttackMode {
    Fast,
    Normal,
    Stealth,
    Aggressive,
}

impl std::str::FromStr for AttackMode {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fast" => Ok(AttackMode::Fast),
            "normal" => Ok(AttackMode::Normal),
            "stealth" => Ok(AttackMode::Stealth),
            "aggressive" => Ok(AttackMode::Aggressive),
            _ => Err(format!("وضع غير صالح: {}", s)),
        }
    }
}