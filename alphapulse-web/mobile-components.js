// Mobile-First Components for AlphaPulse

// Mobile Bottom Sheet Component
class MobileBottomSheet {
    constructor(element, options = {}) {
        this.element = element;
        this.isOpen = false;
        this.threshold = options.threshold || 50; // Swipe threshold
        this.onOpen = options.onOpen || (() => {});
        this.onClose = options.onClose || (() => {});
        
        this.startY = 0;
        this.currentY = 0;
        this.isDragging = false;
        
        this.setupElements();
        this.setupGestures();
    }
    
    setupElements() {
        // Add necessary classes
        this.element.classList.add('mobile-bottom-sheet');
        
        // Find or create handle
        this.handle = this.element.querySelector('.mobile-sheet-handle');
        if (!this.handle) {
            this.handle = document.createElement('div');
            this.handle.className = 'mobile-sheet-handle';
            this.handle.innerHTML = '<div class="mobile-sheet-handle-bar"></div>';
            this.element.insertBefore(this.handle, this.element.firstChild);
        }
    }
    
    setupGestures() {
        // Handle clicks
        this.handle.addEventListener('click', () => this.toggle());
        
        // Touch events for drag
        this.handle.addEventListener('touchstart', (e) => this.onTouchStart(e));
        this.handle.addEventListener('touchmove', (e) => this.onTouchMove(e));
        this.handle.addEventListener('touchend', (e) => this.onTouchEnd(e));
        
        // Mouse events for testing
        this.handle.addEventListener('mousedown', (e) => this.onTouchStart(e));
        document.addEventListener('mousemove', (e) => {
            if (this.isDragging) this.onTouchMove(e);
        });
        document.addEventListener('mouseup', (e) => {
            if (this.isDragging) this.onTouchEnd(e);
        });
    }
    
    onTouchStart(e) {
        this.isDragging = true;
        this.startY = e.touches ? e.touches[0].clientY : e.clientY;
        this.element.style.transition = 'none';
    }
    
    onTouchMove(e) {
        if (!this.isDragging) return;
        
        this.currentY = e.touches ? e.touches[0].clientY : e.clientY;
        const deltaY = this.currentY - this.startY;
        
        if (this.isOpen) {
            // Only allow dragging down when open
            if (deltaY > 0) {
                this.element.style.transform = `translateY(${deltaY}px)`;
            }
        } else {
            // Only allow dragging up when closed
            if (deltaY < 0) {
                const maxHeight = this.element.offsetHeight;
                const translateY = Math.max(deltaY, -maxHeight);
                this.element.style.transform = `translateY(calc(100% + ${translateY}px))`;
            }
        }
    }
    
    onTouchEnd(e) {
        if (!this.isDragging) return;
        
        this.isDragging = false;
        this.element.style.transition = '';
        
        const deltaY = this.currentY - this.startY;
        
        if (Math.abs(deltaY) > this.threshold) {
            if (deltaY > 0 && this.isOpen) {
                this.close();
            } else if (deltaY < 0 && !this.isOpen) {
                this.open();
            } else {
                // Reset position
                this.element.style.transform = '';
            }
        } else {
            // Reset position
            this.element.style.transform = '';
        }
    }
    
    open() {
        this.element.classList.add('open');
        this.isOpen = true;
        this.onOpen();
        
        // Trigger haptic feedback if available
        if (navigator.vibrate) {
            navigator.vibrate(50);
        }
    }
    
    close() {
        this.element.classList.remove('open');
        this.isOpen = false;
        this.onClose();
        
        // Trigger haptic feedback if available
        if (navigator.vibrate) {
            navigator.vibrate(30);
        }
    }
    
    toggle() {
        if (this.isOpen) {
            this.close();
        } else {
            this.open();
        }
    }
}

// Mobile Sidebar Component
class MobileSidebar {
    constructor(sidebar, backdrop, options = {}) {
        this.sidebar = sidebar;
        this.backdrop = backdrop || this.createBackdrop();
        this.isOpen = false;
        this.onOpen = options.onOpen || (() => {});
        this.onClose = options.onClose || (() => {});
        
        this.setupElements();
        this.setupEventListeners();
        this.setupSwipeGestures();
    }
    
    createBackdrop() {
        const backdrop = document.createElement('div');
        backdrop.className = 'mobile-overlay';
        document.body.appendChild(backdrop);
        return backdrop;
    }
    
    setupElements() {
        this.sidebar.classList.add('mobile-sidebar');
        this.backdrop.classList.add('mobile-overlay');
    }
    
    setupEventListeners() {
        // Close on backdrop click
        this.backdrop.addEventListener('click', () => this.close());
        
        // Close on escape key
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape' && this.isOpen) {
                this.close();
            }
        });
    }
    
    setupSwipeGestures() {
        let startX = 0;
        let currentX = 0;
        let isDragging = false;
        
        // Swipe from left edge to open
        document.addEventListener('touchstart', (e) => {
            const touch = e.touches[0];
            if (touch.clientX < 20 && !this.isOpen) {
                startX = touch.clientX;
                isDragging = true;
                this.sidebar.style.transition = 'none';
            }
        });
        
        document.addEventListener('touchmove', (e) => {
            if (!isDragging) return;
            
            currentX = e.touches[0].clientX;
            const deltaX = currentX - startX;
            
            if (deltaX > 0 && deltaX < 300) {
                this.sidebar.style.transform = `translateX(${-100 + (deltaX / 300) * 100}%)`;
                this.backdrop.style.display = 'block';
                this.backdrop.style.opacity = deltaX / 300;
            }
        });
        
        document.addEventListener('touchend', () => {
            if (!isDragging) return;
            
            isDragging = false;
            this.sidebar.style.transition = '';
            this.backdrop.style.transition = 'opacity 0.3s ease';
            
            const deltaX = currentX - startX;
            
            if (deltaX > 150) {
                this.open();
            } else {
                this.sidebar.style.transform = '';
                this.backdrop.style.opacity = '';
                setTimeout(() => {
                    this.backdrop.style.display = '';
                }, 300);
            }
        });
        
        // Swipe on sidebar to close
        this.sidebar.addEventListener('touchstart', (e) => {
            if (!this.isOpen) return;
            startX = e.touches[0].clientX;
            isDragging = true;
        });
        
        this.sidebar.addEventListener('touchmove', (e) => {
            if (!isDragging || !this.isOpen) return;
            
            currentX = e.touches[0].clientX;
            const deltaX = currentX - startX;
            
            if (deltaX < 0) {
                this.sidebar.style.transform = `translateX(${deltaX}px)`;
            }
        });
        
        this.sidebar.addEventListener('touchend', () => {
            if (!isDragging || !this.isOpen) return;
            
            isDragging = false;
            const deltaX = currentX - startX;
            
            if (deltaX < -50) {
                this.close();
            } else {
                this.sidebar.style.transform = '';
            }
        });
    }
    
    open() {
        this.sidebar.classList.add('mobile-visible');
        this.backdrop.classList.add('active');
        this.isOpen = true;
        this.onOpen();
        
        // Prevent body scroll
        document.body.style.overflow = 'hidden';
        
        // Haptic feedback
        if (navigator.vibrate) {
            navigator.vibrate(50);
        }
    }
    
    close() {
        this.sidebar.classList.remove('mobile-visible');
        this.backdrop.classList.remove('active');
        this.isOpen = false;
        this.onClose();
        
        // Restore body scroll
        document.body.style.overflow = '';
        
        // Haptic feedback
        if (navigator.vibrate) {
            navigator.vibrate(30);
        }
    }
    
    toggle() {
        if (this.isOpen) {
            this.close();
        } else {
            this.open();
        }
    }
}

// Mobile Swipe Manager
class MobileSwipeManager {
    constructor(element, options = {}) {
        this.element = element;
        this.threshold = options.threshold || 50;
        this.onSwipeUp = options.onSwipeUp || (() => {});
        this.onSwipeDown = options.onSwipeDown || (() => {});
        this.onSwipeLeft = options.onSwipeLeft || (() => {});
        this.onSwipeRight = options.onSwipeRight || (() => {});
        
        this.setupGestures();
    }
    
    setupGestures() {
        let startX = 0;
        let startY = 0;
        let startTime = 0;
        
        this.element.addEventListener('touchstart', (e) => {
            const touch = e.touches[0];
            startX = touch.clientX;
            startY = touch.clientY;
            startTime = Date.now();
        });
        
        this.element.addEventListener('touchend', (e) => {
            const touch = e.changedTouches[0];
            const endX = touch.clientX;
            const endY = touch.clientY;
            const endTime = Date.now();
            
            const deltaX = endX - startX;
            const deltaY = endY - startY;
            const deltaTime = endTime - startTime;
            
            // Only consider it a swipe if it's fast enough
            if (deltaTime > 500) return;
            
            // Determine swipe direction
            if (Math.abs(deltaX) > Math.abs(deltaY)) {
                // Horizontal swipe
                if (Math.abs(deltaX) > this.threshold) {
                    if (deltaX > 0) {
                        this.onSwipeRight();
                    } else {
                        this.onSwipeLeft();
                    }
                }
            } else {
                // Vertical swipe
                if (Math.abs(deltaY) > this.threshold) {
                    if (deltaY > 0) {
                        this.onSwipeDown();
                    } else {
                        this.onSwipeUp();
                    }
                }
            }
        });
    }
}

// Mobile Pull to Refresh
class MobilePullToRefresh {
    constructor(element, onRefresh, options = {}) {
        this.element = element;
        this.onRefresh = onRefresh;
        this.threshold = options.threshold || 80;
        this.isRefreshing = false;
        
        this.setupElements();
        this.setupGestures();
    }
    
    setupElements() {
        // Create refresh indicator
        this.refreshIndicator = document.createElement('div');
        this.refreshIndicator.className = 'mobile-refresh-indicator';
        this.refreshIndicator.innerHTML = '<div class="mobile-spinner"></div>';
        this.refreshIndicator.style.cssText = `
            position: absolute;
            top: -50px;
            left: 50%;
            transform: translateX(-50%);
            width: 40px;
            height: 40px;
            display: flex;
            align-items: center;
            justify-content: center;
            transition: transform 0.3s ease;
        `;
        
        this.element.style.position = 'relative';
        this.element.appendChild(this.refreshIndicator);
    }
    
    setupGestures() {
        let startY = 0;
        let currentY = 0;
        let isDragging = false;
        
        this.element.addEventListener('touchstart', (e) => {
            if (this.element.scrollTop === 0 && !this.isRefreshing) {
                startY = e.touches[0].clientY;
                isDragging = true;
            }
        });
        
        this.element.addEventListener('touchmove', (e) => {
            if (!isDragging || this.isRefreshing) return;
            
            currentY = e.touches[0].clientY;
            const deltaY = currentY - startY;
            
            if (deltaY > 0 && deltaY < 150) {
                e.preventDefault();
                const progress = Math.min(deltaY / this.threshold, 1);
                this.refreshIndicator.style.transform = `translateX(-50%) translateY(${deltaY}px) rotate(${progress * 360}deg)`;
                this.refreshIndicator.style.opacity = progress;
            }
        });
        
        this.element.addEventListener('touchend', () => {
            if (!isDragging) return;
            
            isDragging = false;
            const deltaY = currentY - startY;
            
            if (deltaY >= this.threshold && !this.isRefreshing) {
                this.refresh();
            } else {
                this.reset();
            }
        });
    }
    
    refresh() {
        this.isRefreshing = true;
        this.refreshIndicator.style.transform = `translateX(-50%) translateY(${this.threshold}px)`;
        this.refreshIndicator.classList.add('refreshing');
        
        // Haptic feedback
        if (navigator.vibrate) {
            navigator.vibrate(100);
        }
        
        // Call refresh handler
        Promise.resolve(this.onRefresh()).then(() => {
            setTimeout(() => {
                this.reset();
                this.isRefreshing = false;
            }, 500);
        });
    }
    
    reset() {
        this.refreshIndicator.style.transform = 'translateX(-50%) translateY(0)';
        this.refreshIndicator.style.opacity = '0';
        this.refreshIndicator.classList.remove('refreshing');
    }
}

// Mobile Navigation Manager
class MobileNavigationManager {
    constructor(options = {}) {
        this.navElement = options.navElement;
        this.menuButton = options.menuButton;
        this.navItems = options.navItems || [];
        
        this.setupNavigation();
    }
    
    setupNavigation() {
        if (this.menuButton) {
            this.menuButton.addEventListener('click', () => {
                this.toggleNav();
            });
        }
        
        // Close nav when clicking outside
        document.addEventListener('click', (e) => {
            if (this.navElement && !this.navElement.contains(e.target) && 
                !this.menuButton.contains(e.target)) {
                this.closeNav();
            }
        });
        
        // Close nav when clicking nav items
        this.navItems.forEach(item => {
            item.addEventListener('click', () => {
                if (window.innerWidth <= 768) {
                    this.closeNav();
                }
            });
        });
    }
    
    toggleNav() {
        if (this.navElement) {
            this.navElement.classList.toggle('mobile-hidden');
        }
    }
    
    closeNav() {
        if (this.navElement) {
            this.navElement.classList.add('mobile-hidden');
        }
    }
    
    openNav() {
        if (this.navElement) {
            this.navElement.classList.remove('mobile-hidden');
        }
    }
}

// Export components
window.MobileComponents = {
    BottomSheet: MobileBottomSheet,
    Sidebar: MobileSidebar,
    SwipeManager: MobileSwipeManager,
    PullToRefresh: MobilePullToRefresh,
    NavigationManager: MobileNavigationManager
};