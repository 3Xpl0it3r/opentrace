/* ═══════════════════════════════════════════════════════════════
   AUTH.JS - 认证与公共 UI 功能
   ═══════════════════════════════════════════════════════════════ */

/* ═══ 认证管理 ═══ */
function checkAuth() {
    const token = localStorage.getItem('token');
    if (!token && !window.location.pathname.includes('login')) {
        window.location.href = '/login.html';
        return false;
    }
    return true;
}

function logout() {
    localStorage.removeItem('token');
    localStorage.removeItem('user');
    window.location.href = '/login.html';
}

function getUser() {
    const user = localStorage.getItem('user');
    return user ? JSON.parse(user) : null;
}

/* ═══ 主题切换 ═══ */
var theme = localStorage.getItem('ot-theme') || 'dark';

function applyTheme() {
    document.documentElement.className = theme;
    var moon = document.getElementById('icon-moon');
    var sun = document.getElementById('icon-sun');
    if (moon && sun) {
        moon.style.display = theme === 'dark' ? '' : 'none';
        sun.style.display = theme === 'light' ? '' : 'none';
    }
}

function toggleTheme() {
    theme = theme === 'dark' ? 'light' : 'dark';
    localStorage.setItem('ot-theme', theme);
    applyTheme();
}

/* ═══ 用户菜单 ═══ */
function initUserMenu() {
    var user = getUser();
    if (!user) return;
    
    var avatarEl = document.getElementById('user-avatar');
    var ddAvatarEl = document.getElementById('dd-av');
    var ddNameEl = document.getElementById('dd-name');
    var ddRoleEl = document.getElementById('dd-role');
    
    var avatarChar = user.avatar || (user.username ? user.username.charAt(0).toUpperCase() : 'U');
    
    if (avatarEl) avatarEl.textContent = avatarChar;
    if (ddAvatarEl) ddAvatarEl.textContent = avatarChar;
    if (ddNameEl) ddNameEl.textContent = user.username || 'User';
    if (ddRoleEl) ddRoleEl.textContent = user.role || '用户';
}

function toggleDropdown() {
    var dd = document.getElementById('user-dropdown');
    var btn = document.getElementById('user-avatar');
    if (!dd || !btn) return;
    
    var isOpen = dd.classList.toggle('show');
    btn.setAttribute('aria-expanded', isOpen);
}

function initUserMenuEvents() {
    document.addEventListener('click', function(e) {
        var menu = document.getElementById('user-menu');
        if (menu && !menu.contains(e.target)) {
            var dd = document.getElementById('user-dropdown');
            var btn = document.getElementById('user-avatar');
            if (dd) dd.classList.remove('show');
            if (btn) btn.setAttribute('aria-expanded', 'false');
        }
    });
}

/* ═══ 时钟更新 ═══ */
function updateClock() {
    var clockEl = document.getElementById('clock');
    if (clockEl) {
        clockEl.textContent = new Date().toLocaleTimeString('zh-CN', {hour12: false});
    }
}

function initClock() {
    updateClock();
    setInterval(updateClock, 1000);
}

/* ═══ 模态框管理 ═══ */
var lastFocusedElement = null;

function openModal(type) {
    lastFocusedElement = document.activeElement;
    var overlay = document.getElementById('modal-' + type);
    if (!overlay) return;
    
    overlay.classList.add('show');
    var firstInput = overlay.querySelector('input, button:not(.modal-close)');
    if (firstInput) {
        setTimeout(function() { firstInput.focus(); }, 50);
    }
}

function closeModal(type) {
    var overlay = document.getElementById('modal-' + type);
    if (!overlay) return;
    
    overlay.classList.remove('show');
    if (lastFocusedElement) {
        lastFocusedElement.focus();
        lastFocusedElement = null;
    }
}

function initModalEvents() {
    // 点击外部关闭模态框
    document.querySelectorAll('.modal-overlay').forEach(function(el) {
        el.addEventListener('click', function(e) {
            if (e.target === el) {
                el.classList.remove('show');
            }
        });
    });
    
    // Escape 键关闭模态框
    document.addEventListener('keydown', function(e) {
        if (e.key === 'Escape') {
            document.querySelectorAll('.modal-overlay.show').forEach(function(el) {
                el.classList.remove('show');
            });
        }
        
        // 焦点陷阱
        var openModalEl = document.querySelector('.modal-overlay.show .modal');
        if (openModalEl) {
            var focusable = openModalEl.querySelectorAll('input, select, button, [tabindex]:not([tabindex="-1"])');
            if (!focusable.length) return;
            
            var first = focusable[0];
            var last = focusable[focusable.length - 1];
            
            if (e.key === 'Tab') {
                if (e.shiftKey && document.activeElement === first) {
                    e.preventDefault();
                    last.focus();
                } else if (!e.shiftKey && document.activeElement === last) {
                    e.preventDefault();
                    first.focus();
                }
            }
        }
    });
}

/* ═══ 表单错误处理 ═══ */
function clearFormErrors() {
    document.querySelectorAll('.form-error').forEach(function(el) {
        el.classList.remove('show');
    });
    document.querySelectorAll('.form-input.error').forEach(function(el) {
        el.classList.remove('error');
    });
    var inlineError = document.getElementById('create-error');
    if (inlineError) inlineError.classList.remove('show');
}

function showFieldError(inputId) {
    var input = document.getElementById(inputId);
    var fieldName = inputId.split('-').pop();
    var errorEl = document.getElementById('err-' + fieldName);
    
    if (input) input.classList.add('error');
    if (errorEl) errorEl.classList.add('show');
}

function showInlineError(message) {
    var errorEl = document.getElementById('create-error');
    if (errorEl) {
        errorEl.textContent = message;
        errorEl.classList.add('show');
    }
}

/* ═══ 初始化公共功能 ═══ */
function initCommon() {
    applyTheme();
    initUserMenu();
    initUserMenuEvents();
    initClock();
    initModalEvents();
}

// 页面加载完成后初始化
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initCommon);
} else {
    initCommon();
}
