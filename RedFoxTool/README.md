cat > README.md << 'EOF'
# 🦊 RedFoxTool

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/yourusername/redfox-tool.svg)](https://github.com/yourusername/redfox-tool/stargazers)
[![GitHub Issues](https://img.shields.io/github/issues/yourusername/redfox-tool.svg)](https://github.com/yourusername/redfox-tool/issues)

**أداة تخمين كلمات مرور فائقة السرعة مكتوبة بلغة Rust للأداء الأمثل**

## ✨ المميزات الرئيسية

- ⚡ **سرعة فائقة**: معالجة متوازية باستخدام Tokio و Rayon
- 🎯 **دقة عالية**: خوارزميات ذكية للتخمين الأمثل
- 📊 **تقارير متعددة**: JSON, HTML, CSV, TXT, XML
- 🔧 **قابلة للتخصيص**: 4 أوضاع هجوم مختلفة
- 🛡️ **آمنة**: تحقق من الصلاحيات وحماية متقدمة
- 📱 **متعددة الأنظمة**: تعمل على Linux, Windows, macOS

## 🚀 البداية السريعة

### التثبيت
```bash
# التجميع من المصدر
git clone https://github.com/yourusername/redfox-tool.git
cd redfox-tool
cargo build --release
sudo cp target/release/redfox-tool /usr/local/bin/redfox