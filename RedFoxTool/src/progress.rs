//! شريط التقدم ونظام التتبع
//! يوفر تتبعًا مرئيًا للتقدم

use std::sync::Arc;
use std::time::{Instant, Duration};
use indicatif::{ProgressBar, ProgressStyle, MultiProgress, HumanDuration};
use tokio::sync::RwLock;
use colored::Colorize;

/// متعقب التقدم
pub struct ProgressTracker {
    pb: Option<ProgressBar>,
    start_time: Instant,
    total_items: usize,
    completed: usize,
    last_update: Instant,
    speed_history: Vec<f64>,
}

impl ProgressTracker {
    /// إنشاء متعقب جديد
    pub fn new(total_items: usize) -> Self {
        let pb = if total_items > 100 {
            let pb = ProgressBar::new(total_items as u64);
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
        
        Self {
            pb,
            start_time: Instant::now(),
            total_items,
            completed: 0,
            last_update: Instant::now(),
            speed_history: Vec::new(),
        }
    }
    
    /// تحديث التقدم
    pub fn update(&mut self, increment: usize) {
        self.completed += increment;
        
        if let Some(pb) = &self.pb {
            pb.inc(increment as u64);
            
            // تحديث الرسالة كل 1000 عنصر
            if self.completed % 1000 == 0 {
                let elapsed = self.start_time.elapsed();
                let speed = self.completed as f64 / elapsed.as_secs_f64();
                pb.set_message(format!("{:.1}/s", speed));
                
                // حفظ السرعة للتاريخ
                self.speed_history.push(speed);
                if self.speed_history.len() > 10 {
                    self.speed_history.remove(0);
                }
            }
        }
        
        self.last_update = Instant::now();
    }
    
    /// إكمال التقدم
    pub fn finish(&mut self) {
        if let Some(pb) = &self.pb {
            pb.finish_with_message("اكتمل!");
        }
        
        let elapsed = self.start_time.elapsed();
        let speed = self.completed as f64 / elapsed.as_secs_f64();
        
        println!(
            "{}: {} عنصر في {} ({:.1} عنصر/ثانية)",
            "اكتمل".bright_green(),
            self.completed,
            HumanDuration(elapsed),
            speed
        );
    }
    
    /// الحصول على النسبة المئوية للتقدم
    pub fn percentage(&self) -> f64 {
        if self.total_items == 0 {
            100.0
        } else {
            (self.completed as f64 / self.total_items as f64) * 100.0
        }
    }
    
    /// الحصول على الوقت المتبقي
    pub fn eta(&self) -> Option<Duration> {
        if self.completed == 0 {
            return None;
        }
        
        let elapsed = self.start_time.elapsed();
        let items_per_second = self.completed as f64 / elapsed.as_secs_f64();
        
        if items_per_second > 0.0 {
            let remaining = (self.total_items - self.completed) as f64 / items_per_second;
            Some(Duration::from_secs_f64(remaining))
        } else {
            None
        }
    }
    
    /// الحصول على متوسط السرعة
    pub fn average_speed(&self) -> f64 {
        if self.speed_history.is_empty() {
            let elapsed = self.start_time.elapsed();
            if elapsed.as_secs() > 0 {
                self.completed as f64 / elapsed.as_secs_f64()
            } else {
                0.0
            }
        } else {
            self.speed_history.iter().sum::<f64>() / self.speed_history.len() as f64
        }
    }
    
    /// التحقق مما إذا كان التقدم متوقفًا
    pub fn is_stalled(&self, threshold: Duration) -> bool {
        Instant::now().duration_since(self.last_update) > threshold
    }
    
    /// عرض حالة التقدم
    pub fn display_status(&self) {
        let percentage = self.percentage();
        let elapsed = self.start_time.elapsed();
        let speed = self.average_speed();
        
        println!(
            "{}: {:.1}% | {} / {} | {:.1}/ثانية | {}",
            "التقدم".bright_cyan(),
            percentage,
            self.completed,
            self.total_items,
            speed,
            HumanDuration(elapsed)
        );
        
        if let Some(eta) = self.eta() {
            println!("{}: {}", "الوقت المتبقي".bright_yellow(), HumanDuration(eta));
        }
    }
}

/// شريط تقدم متعدد (للمهام المتعددة)
pub struct MultiProgressTracker {
    multi: MultiProgress,
    trackers: Vec<Arc<RwLock<ProgressTracker>>>,
}

impl MultiProgressTracker {
    /// إنشاء متعقب متعدد
    pub fn new() -> Self {
        Self {
            multi: MultiProgress::new(),
            trackers: Vec::new(),
        }
    }
    
    /// إضافة مهمة جديدة
    pub fn add_task(&mut self, name: &str, total_items: usize) -> Arc<RwLock<ProgressTracker>> {
        let pb = self.multi.add(ProgressBar::new(total_items as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template(&format!("{{spinner:.green}} {} [{{bar:40.cyan/blue}}] {{pos}}/{{len}} ({{eta}})", name))
                .unwrap()
                .progress_chars("#>-")
        );
        
        let tracker = ProgressTracker {
            pb: Some(pb),
            start_time: Instant::now(),
            total_items,
            completed: 0,
            last_update: Instant::now(),
            speed_history: Vec::new(),
        };
        
        let tracker_arc = Arc::new(RwLock::new(tracker));
        self.trackers.push(Arc::clone(&tracker_arc));
        
        tracker_arc
    }
    
    /// إنهاء جميع المهام
    pub fn finish_all(&self) {
        self.multi.clear().unwrap();
    }
}

/// شريط تقدم مبسط بدون مؤشرات
pub struct SimpleProgress {
    total: usize,
    current: usize,
    start_time: Instant,
    last_print: Instant,
    print_interval: Duration,
}

impl SimpleProgress {
    /// إنشاء شريط تقدم مبسط
    pub fn new(total: usize) -> Self {
        Self {
            total,
            current: 0,
            start_time: Instant::now(),
            last_print: Instant::now(),
            print_interval: Duration::from_secs(1),
        }
    }
    
    /// تحديث التقدم
    pub fn update(&mut self, increment: usize) {
        self.current += increment;
        
        let now = Instant::now();
        if now.duration_since(self.last_print) >= self.print_interval {
            self.print_status();
            self.last_print = now;
        }
    }
    
    /// طباعة الحالة
    fn print_status(&self) {
        let elapsed = self.start_time.elapsed();
        let percentage = (self.current as f64 / self.total as f64) * 100.0;
        let speed = self.current as f64 / elapsed.as_secs_f64();
        
        print!(
            "\r{}: {:.1}% | {}/{} | {:.1}/ثانية | {}",
            "تقدم".bright_cyan(),
            percentage,
            self.current,
            self.total,
            speed,
            HumanDuration(elapsed)
        );
        
        if self.current < self.total {
            if let Some(eta) = self.estimate_eta() {
                print!(" | {}: {}", "متبقي".bright_yellow(), HumanDuration(eta));
            }
        }
        
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
    }
    
    /// تقدير الوقت المتبقي
    fn estimate_eta(&self) -> Option<Duration> {
        if self.current == 0 {
            return None;
        }
        
        let elapsed = self.start_time.elapsed();
        let speed = self.current as f64 / elapsed.as_secs_f64();
        
        if speed > 0.0 {
            let remaining = (self.total - self.current) as f64 / speed;
            Some(Duration::from_secs_f64(remaining))
        } else {
            None
        }
    }
    
    /// إنهاء التقدم
    pub fn finish(&mut self) {
        self.current = self.total;
        self.print_status();
        println!(); // سطر جديد بعد الانتهاء
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_progress_tracker() {
        let mut tracker = ProgressTracker::new(1000);
        
        assert_eq!(tracker.percentage(), 0.0);
        
        tracker.update(100);
        assert_eq!(tracker.percentage(), 10.0);
        
        tracker.update(400);
        assert_eq!(tracker.percentage(), 50.0);
        
        tracker.finish();
    }
    
    #[test]
    fn test_simple_progress() {
        let mut progress = SimpleProgress::new(500);
        
        for i in 0..5 {
            progress.update(100);
            thread::sleep(Duration::from_millis(100));
        }
        
        progress.finish();
    }
}