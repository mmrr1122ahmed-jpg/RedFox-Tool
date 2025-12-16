#!/bin/bash

# سكريبت تحديث RedFoxTool

set -euo pipefail

# الألوان
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# متغيرات
INSTALL_DIR="/opt/redfox-tool"
BACKUP_DIR="/opt/redfox-backup"
REPO_URL="https://github.com/redfox-security/redfox-tool.git"

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

# التحقق من التثبيت
check_installation() {
    if [[ ! -d "$INSTALL_DIR" ]]; then
        log_error "RedFoxTool غير مثبت في $INSTALL_DIR"
        exit 1
    fi
    
    if ! command -v redfox &> /dev/null; then
        log_error "الملف التنفيذي لـ RedFoxTool غير موجود"
        exit 1
    fi
}

# إنشاء نسخة احتياطية
create_backup() {
    log_info "إنشاء نسخة احتياطية..."
    
    TIMESTAMP=$(date +%Y%m%d_%H%M%S)
    BACKUP_PATH="$BACKUP_DIR/redfox_$TIMESTAMP"
    
    mkdir -p "$BACKUP_PATH"
    
    # نسخ الملفات المهمة
    cp -r "$INSTALL_DIR" "$BACKUP_PATH/redfox-tool"
    cp /usr/local/bin/redfox "$BACKUP_PATH/"
    cp -r /etc/redfox "$BACKUP_PATH/config"
    
    log_success "تم إنشاء النسخة الاحتياطية في: $BACKUP_PATH"
}

# تحديث من Git
update_from_git() {
    log_info "تحديث من مستودع Git..."
    
    cd "$INSTALL_DIR"
    
    # حفظ التغييرات المحلية
    if git status --porcelain | grep -q "."; then
        log_warning "توجد تغييرات محلية، يتم حفظها..."
        git stash
    fi
    
    # سحب التحديثات
    git pull origin main
    
    # استعادة التغييرات المحلية
    if git stash list | grep -q "stash"; then
        git stash pop
    fi
    
    log_success "تم سحب التحديثات"
}

# بناء المشروع
build_project() {
    log_info "بناء المشروع..."
    
    cd "$INSTALL_DIR"
    
    # تحديث حزم Rust
    log_info "تحديث حزم Rust..."
    cargo update
    
    # البناء
    log_info "بناء RedFoxTool..."
    cargo build --release --features "full"
    
    # نسخ الملف التنفيذي
    cp "target/release/redfox-tool" /usr/local/bin/redfox
    chmod +x /usr/local/bin/redfox
    
    log_success "تم البناء والتحديث"
}

# تحديث التكوينات
update_configs() {
    log_info "تحديث ملفات التكوين..."
    
    # نسخ ملفات التكوين الجديدة
    if [[ -f "$INSTALL_DIR/configs/redfox.conf" ]]; then
        cp "$INSTALL_DIR/configs/redfox.conf" /etc/redfox/redfox.conf.new
        log_warning "ملف تكوين جديد متاح: /etc/redfox/redfox.conf.new"
        log_warning "قم بمراجعة التغييرات قبل تطبيقها"
    fi
    
    # تحديث قوائم الكلمات
    if [[ -d "$INSTALL_DIR/wordlists" ]]; then
        cp -r "$INSTALL_DIR/wordlists"/* /usr/share/redfox/wordlists/ 2>/dev/null || true
    fi
    
    log_success "تم تحديث الملفات المساعدة"
}

# اختبار التحديث
test_update() {
    log_info "اختبار التحديث..."
    
    if redfox --version &> /dev/null; then
        log_success "RedFoxTool يعمل بشكل صحيح"
        
        # عرض الإصدار
        VERSION=$(redfox --version 2>/dev/null || echo "unknown")
        log_info "الإصدار الحالي: $VERSION"
    else
        log_error "فشل اختبار RedFoxTool"
        log_warning "جاري استعادة النسخة الاحتياطية..."
        restore_backup
        exit 1
    fi
}

# استعادة النسخة الاحتياطية
restore_backup() {
    log_warning "استعادة من النسخة الاحتياطية..."
    
    LATEST_BACKUP=$(ls -td "$BACKUP_DIR"/*/ | head -1)
    
    if [[ -n "$LATEST_BACKUP" ]]; then
        log_info "استعادة من: $LATEST_BACKUP"
        
        # استعادة الملفات
        cp -r "$LATEST_BACKUP/redfox-tool"/* "$INSTALL_DIR/"
        cp "$LATEST_BACKUP/redfox" /usr/local/bin/redfox
        cp -r "$LATEST_BACKUP/config"/* /etc/redfox/
        
        log_success "تم الاستعادة"
    else
        log_error "لا توجد نسخ احتياطية للاستعادة"
    fi
}

# عرض ملخص التحديث
show_summary() {
    echo -e "\n${GREEN}"
    cat << "EOF"
╔═══════════════════════════════════════════════════════════╗
║              RedFoxTool Update Complete!                  ║
╚═══════════════════════════════════════════════════════════╝
EOF
    echo -e "${NC}"
    
    log_success "التحديث اكتمل بنجاح!"
    echo ""
    log_info "التغييرات:"
    echo "  • تم تحديث الكود المصدري"
    echo "  • تم بناء المشروع بأحدث التغييرات"
    echo "  • تم تحديث الملف التنفيذي"
    echo "  • تم إنشاء نسخة احتياطية"
    echo ""
    log_warning "ملاحظة: قد تحتاج لمراجعة ملفات التكوين الجديدة"
    echo ""
    log_info "استخدم الأمر التالي للتحقق:"
    echo "  redfox --version"
}

# الدالة الرئيسية
main() {
    echo -e "${BLUE}"
    cat << "EOF"
    ██████╗ ███████╗██████╗ ███████╗ ██████╗ ██╗  ██╗
    ██╔══██╗██╔════╝██╔══██╗██╔════╝██╔═══██╗╚██╗██╔╝
    ██████╔╝█████╗  ██║  ██║█████╗  ██║   ██║ ╚███╔╝ 
    ██╔══██╗██╔══╝  ██║  ██║██╔══╝  ██║   ██║ ██╔██╗ 
    ██║  ██║███████╗██████╔╝██║     ╚██████╔╝██╔╝ ██╗
    ╚═╝  ╚═╝╚══════╝╚═════╝ ╚═╝      ╚═════╝ ╚═╝  ╚═╝
    
    RedFoxTool Updater v1.0
    ======================
EOF
    echo -e "${NC}"
    
    check_installation
    create_backup
    update_from_git
    build_project
    update_configs
    test_update
    show_summary
}

# تنفيذ
main "$@"