// SpecterOS Website JavaScript

// Mobile Navigation Toggle
document.addEventListener('DOMContentLoaded', () => {
    const navToggle = document.querySelector('.nav-toggle');
    const navMenu = document.querySelector('.nav-menu');
    
    if (navToggle) {
        navToggle.addEventListener('click', () => {
            navMenu.classList.toggle('active');
        });
        
        // Close menu when clicking outside
        document.addEventListener('click', (e) => {
            if (!navToggle.contains(e.target) && !navMenu.contains(e.target)) {
                navMenu.classList.remove('active');
            }
        });
    }
    
    // Smooth scroll for anchor links
    document.querySelectorAll('a[href^="#"]').forEach(anchor => {
        anchor.addEventListener('click', function (e) {
            e.preventDefault();
            const target = document.querySelector(this.getAttribute('href'));
            if (target) {
                target.scrollIntoView({
                    behavior: 'smooth',
                    block: 'start'
                });
                // Close mobile menu if open
                navMenu.classList.remove('active');
            }
        });
    });
    
    // Navbar scroll effect
    const navbar = document.querySelector('.navbar');
    let lastScroll = 0;
    
    window.addEventListener('scroll', () => {
        const currentScroll = window.pageYOffset;
        
        if (currentScroll > 100) {
            navbar.style.background = 'rgba(15, 15, 15, 0.98)';
            navbar.style.boxShadow = '0 2px 20px rgba(0, 0, 0, 0.5)';
        } else {
            navbar.style.background = 'rgba(15, 15, 15, 0.95)';
            navbar.style.boxShadow = 'none';
        }
        
        lastScroll = currentScroll;
    });
    
    // Intersection Observer for animations
    const observerOptions = {
        threshold: 0.1,
        rootMargin: '0px 0px -50px 0px'
    };
    
    const observer = new IntersectionObserver((entries) => {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                entry.target.style.opacity = '1';
                entry.target.style.transform = 'translateY(0)';
            }
        });
    }, observerOptions);
    
    // Observe feature cards and other elements
    document.querySelectorAll('.feature-card, .community-card, .download-card').forEach(el => {
        el.style.opacity = '0';
        el.style.transform = 'translateY(20px)';
        el.style.transition = 'opacity 0.6s ease, transform 0.6s ease';
        observer.observe(el);
    });
    
    // Dynamic GitHub stats (if available)
    fetchGitHubStats();
    
    // Download button analytics (placeholder)
    setupDownloadTracking();
});

// Fetch GitHub repository stats
async function fetchGitHubStats() {
    try {
        const response = await fetch('https://api.github.com/repos/Sskibls/Specteros-OS');
        const data = await response.json();
        
        if (data.stargazers_count !== undefined) {
            // Update stats if we have a stars element
            const starsElement = document.querySelector('.github-stars');
            if (starsElement) {
                starsElement.textContent = data.stargazers_count;
            }
        }
    } catch (error) {
        console.log('GitHub stats not available');
    }
}

// Track download button clicks
function setupDownloadTracking() {
    const downloadButtons = document.querySelectorAll('[href="#download"]');
    
    downloadButtons.forEach(button => {
        button.addEventListener('click', (e) => {
            // Analytics placeholder
            console.log('Download button clicked');
            
            // You can add Google Analytics, Plausible, or other tracking here
            // gtag('event', 'click', { event_category: 'download', event_label: 'homepage' });
        });
    });
}

// Copy checksum to clipboard
function copyChecksum() {
    const checksum = document.querySelector('.checksum-info code');
    if (checksum) {
        navigator.clipboard.writeText(checksum.textContent).then(() => {
            // Show copy feedback
            const originalText = checksum.textContent;
            checksum.textContent = 'Copied!';
            setTimeout(() => {
                checksum.textContent = originalText;
            }, 2000);
        });
    }
}

// Add copy button to checksum
document.addEventListener('DOMContentLoaded', () => {
    const checksumInfo = document.querySelector('.checksum-info');
    if (checksumInfo) {
        const copyButton = document.createElement('button');
        copyButton.textContent = 'Copy';
        copyButton.className = 'btn btn-secondary';
        copyButton.style.marginTop = '10px';
        copyButton.style.fontSize = '0.85rem';
        copyButton.style.padding = '8px 16px';
        copyButton.onclick = copyChecksum;
        checksumInfo.appendChild(copyButton);
    }
});

// Version comparison (for update notifications)
function checkForUpdates(currentVersion, latestVersion) {
    const current = currentVersion.split('.').map(Number);
    const latest = latestVersion.split('.').map(Number);
    
    for (let i = 0; i < Math.max(current.length, latest.length); i++) {
        const c = current[i] || 0;
        const l = latest[i] || 0;
        if (l > c) return true;
        if (l < c) return false;
    }
    return false;
}

// Show update notification if new version available
function showUpdateNotification() {
    const currentVersion = '0.1.0';
    // Fetch latest version from GitHub releases
    fetch('https://api.github.com/repos/Sskibls/Specteros-OS/releases/latest')
        .then(response => response.json())
        .then(data => {
            const latestVersion = data.tag_name.replace('v', '');
            if (checkForUpdates(currentVersion, latestVersion)) {
                // Show notification banner
                console.log(`New version available: ${latestVersion}`);
            }
        })
        .catch(() => {});
}

// Initialize on load
window.addEventListener('load', () => {
    showUpdateNotification();
});

// Console easter egg
console.log('%c👻 SpecterOS', 'font-size: 24px; font-weight: bold; color: #7C3AED;');
console.log('%cPrivacy-First Linux Distribution', 'font-size: 14px; color: #14B8A6;');
console.log('%cBuilt with ❤️ for privacy and security', 'font-size: 12px; color: #A3A3A3;');
console.log('%cJoin us: https://github.com/Sskibls/Specteros-OS', 'font-size: 12px; color: #14B8A6;');
