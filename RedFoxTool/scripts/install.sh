#!/bin/bash

# RedFoxTool Installer for Kali Linux
# إصدار التثبيت الرسمي

set -euo pipefail

# الألوان
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# متغيرات
VERSION="1.0.0"
INSTALL_DIR="/opt/redfox-tool"
BIN_DIR="/usr/local/bin"
CONFIG_DIR="/etc/redfox"
LOG_DIR="/var/log/redfox"
DATA_DIR="/usr/share/redfox"

# دالة لعرض الرسائل
log_info() {
    echo -e "${BLUE}[*]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[+]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

log_error() {
    echo -e "${RED}[-]${NC} $1"
}

# التحقق من صلاحيات الجذر
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "يجب تشغيل السكريبت كـ root"
        echo "استخدم: sudo ./install.sh"
        exit 1
    fi
}

# التحقق من نظام التشغيل
check_os() {
    if [[ ! -f /etc/os-release ]]; then
        log_error "لم يتم التعرف على نظام التشغيل"
        exit 1
    fi
    
    source /etc/os-release
    
    if [[ "$ID" != "kali" ]] && [[ "$ID_LIKE" != *"debian"* ]]; then
        log_warning "هذا السكريبت مصمم لكالي لينكس وأنظمة دبيان"
        read -p "هل تريد المتابعة؟ (y/n): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
    
    log_info "نظام التشغيل: $PRETTY_NAME"
}

# عرض البانر
show_banner() {
    clear
    echo -e "${RED}"
    cat << "EOF"
    ██████╗ ███████╗██████╗ ███████╗ ██████╗ ██╗  ██╗
    ██╔══██╗██╔════╝██╔══██╗██╔════╝██╔═══██╗╚██╗██╔╝
    ██████╔╝█████╗  ██║  ██║█████╗  ██║   ██║ ╚███╔╝ 
    ██╔══██╗██╔══╝  ██║  ██║██╔══╝  ██║   ██║ ██╔██╗ 
    ██║  ██║███████╗██████╔╝██║     ╚██████╔╝██╔╝ ██╗
    ╚═╝  ╚═╝╚══════╝╚═════╝ ╚═╝      ╚═════╝ ╚═╝  ╚═╝
    
    RedFoxTool v1.0.0 - Ultra Fast Password Auditor
    ===============================================
EOF
    echo -e "${NC}"
}

# تحديث النظام
update_system() {
    log_info "تحديث النظام..."
    apt update && apt upgrade -y
    log_success "تم تحديث النظام"
}

# تثبيت المتطلبات
install_dependencies() {
    log_info "تثبيت المتطلبات..."
    
    # تحديث قائمة الحزم
    apt update
    
    # تثبيت Rust إذا لم يكن مثبتًا
    if ! command -v rustc &> /dev/null; then
        log_info "تثبيت Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    
    # تثبيت حزم النظام
    apt install -y \
        build-essential \
        libssl-dev \
        pkg-config \
        cmake \
        git \
        curl \
        wget \
        python3 \
        python3-pip \
        jq \
        sqlite3 \
        zip \
        unzip \
        screen \
        tmux
    
    log_success "تم تثبيت المتطلبات"
}

# تثبيت RedFoxTool
install_redfox() {
    log_info "تثبيت RedFoxTool..."
    
    # إنشاء المجلدات
    mkdir -p "$INSTALL_DIR"
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$LOG_DIR"
    mkdir -p "$DATA_DIR/wordlists"
    mkdir -p "$DATA_DIR/templates"
    
    # نسخ الملفات المصدرية
    cp -r src/* "$INSTALL_DIR/"
    cp Cargo.toml Cargo.lock "$INSTALL_DIR/"
    cp -r configs/* "$CONFIG_DIR/"
    cp -r scripts/* "$INSTALL_DIR/scripts/"
    
    # إنشاء ملفات التكوين
    cat > "$CONFIG_DIR/redfox.conf" << EOF
# إعدادات RedFoxTool
[general]
name = "RedFoxTool"
version = "$VERSION"
log_level = "info"
log_file = "$LOG_DIR/redfox.log"

[scanning]
default_threads = 20
default_timeout = 30
max_retries = 3
rate_limit = 100
user_agent = "RedFoxTool/1.0"

[wordlists]
default_users = "$DATA_DIR/wordlists/common_users.txt"
default_passwords = "$DATA_DIR/wordlists/common_passwords.txt"
seclists_path = "/usr/share/seclists"
rockyou_path = "/usr/share/wordlists/rockyou.txt"

[output]
default_format = "json"
save_results = true
results_dir = "$LOG_DIR/results"
auto_open = false
EOF
    
    # إنشاء قوائم الكلمات الافتراضية
    cat > "$DATA_DIR/wordlists/common_users.txt" << EOF
admin
administrator
root
user
test
guest
admin1
admin123
superuser
EOF
    
    cat > "$DATA_DIR/wordlists/common_passwords.txt" << EOF
admin
123456
password
12345678
123456789
12345
1234
123
qwerty
password1
EOF
    
    # بناء المشروع
    log_info "بناء RedFoxTool..."
    cd "$INSTALL_DIR"
    cargo build --release --features "full"
    
    # نسخ الملفات التنفيذية
    cp "target/release/redfox-tool" "$BIN_DIR/redfox"
    chmod +x "$BIN_DIR/redfox"
    
    # إنشاء رابط للوثائق
    ln -sf "$INSTALL_DIR/README.md" "/usr/share/doc/redfox/README.md"
    
    log_success "تم تثبيت RedFoxTool"
}

# تثبيت سكريبتات الخدمة
install_services() {
    log_info "تثبيت سكريبتات الخدمة..."
    
    # إنشاء سكريبت التحديث التلقائي
    cat > /etc/cron.daily/redfox-update << 'EOF'
#!/bin/bash
# تحديث RedFoxTool يوميًا

REDFOX_DIR="/opt/redfox-tool"
LOG_FILE="/var/log/redfox/update.log"

cd "$REDFOX_DIR" || exit 1

echo "[$(date)] Checking for updates..." >> "$LOG_FILE"

if git pull | grep -q "Already up to date"; then
    echo "RedFoxTool is up to date" >> "$LOG_FILE"
else
    echo "Updating RedFoxTool..." >> "$LOG_FILE"
    cargo build --release --features "full"
    cp "target/release/redfox-tool" /usr/local/bin/redfox
    echo "Update completed at $(date)" >> "$LOG_FILE"
fi
EOF
    
    chmod +x /etc/cron.daily/redfox-update
    
    # إنشاء سكريبت تنظيف السجلات
    cat > /etc/cron.weekly/redfox-cleanup << 'EOF'
#!/bin/bash
# تنظيف سجلات RedFoxTool أسبوعيًا

LOG_DIR="/var/log/redfox"
MAX_AGE_DAYS=30

find "$LOG_DIR" -type f -name "*.log" -mtime +$MAX_AGE_DAYS -delete
find "$LOG_DIR/results" -type f -mtime +$MAX_AGE_DAYS -delete

echo "[$(date)] Cleaned old RedFoxTool logs" >> "$LOG_DIR/maintenance.log"
EOF
    
    chmod +x /etc/cron.weekly/redfox-cleanup
    
    log_success "تم تثبيت سكريبتات الخدمة"
}

# إعداد صلاحيات الملفات
setup_permissions() {
    log_info "إعداد الصلاحيات..."
    
    # إعداد مالك المجلدات
    chown -R root:root "$INSTALL_DIR"
    chown -R root:root "$CONFIG_DIR"
    chown -R root:root "$DATA_DIR"
    
    # صلاحيات المجلدات
    chmod 755 "$INSTALL_DIR"
    chmod 755 "$CONFIG_DIR"
    chmod 755 "$DATA_DIR"
    chmod 755 "$LOG_DIR"
    
    # صلاحيات الملفات التنفيذية
    chmod 755 "$BIN_DIR/redfox"
    
    # صلاحيات السجلات
    chmod 666 "$LOG_DIR"/*.log 2>/dev/null || true
    
    log_success "تم إعداد الصلاحيات"
}

# اختبار التثبيت
test_installation() {
    log_info "اختبار التثبيت..."
    
    if command -v redfox &> /dev/null; then
        VERSION_OUTPUT=$(redfox --version 2>/dev/null || true)
        if [[ "$VERSION_OUTPUT" == *"RedFoxTool"* ]]; then
            log_success "RedFoxTool مثبت بنجاح"
            log_success "الإصدار: $VERSION_OUTPUT"
        else
            log_warning "تم تثبيت RedFoxTool ولكن اختبار الإصدار فشل"
        fi
    else
        log_error "فشل تثبيت RedFoxTool"
        exit 1
    fi
    
    # اختبار بسيط
    if [[ -f "$DATA_DIR/wordlists/common_users.txt" ]]; then
        log_success "قوائم الكلمات مثبتة"
    fi
    
    if [[ -f "$CONFIG_DIR/redfox.conf" ]]; then
        log_success "ملف التكوين مثبت"
    fi
}

# عرض رسالة النجاح
show_success() {
    echo -e "\n${GREEN}"
    cat << "EOF"
╔═══════════════════════════════════════════════════════════╗
║            RedFoxTool Installation Complete!              ║
╚═══════════════════════════════════════════════════════════╝
EOF
    echo -e "${NC}"
    
    log_success "التثبيت اكتمل بنجاح!"
    echo ""
    log_info "المسارات المهمة:"
    echo "  • الملف التنفيذي: /usr/local/bin/redfox"
    echo "  • دليل التثبيت: $INSTALL_DIR"
    echo "  · التكوينات: $CONFIG_DIR"
    echo "  · السجلات: $LOG_DIR"
    echo "  · البيانات: $DATA_DIR"
    echo ""
    log_info "أمثلة للاستخدام:"
    echo "  redfox --help"
    echo "  redfox scan --url http://test.com/login -U admin -P passwords.txt"
    echo "  redfox benchmark --url http://test.com --users users.txt --passwords passwords.txt"
    echo ""
    log_warning "هام: استخدم الأداة فقط لأغراض اختبار الاختراق المصرح بها!"
}

# الدالة الرئيسية
main() {
    show_banner
    check_root
    check_os
    
    log_info "بدء تثبيت RedFoxTool v$VERSION"
    echo ""
    
    update_system
    install_dependencies
    install_redfox
    install_services
    setup_permissions
    test_installation
    show_success
}

# تنفيذ الدالة الرئيسية
main "$@"