import React, { useEffect, useState } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { useAppStore } from '../../store/useAppStore';
import { MenuIcon, CloseIcon, SunIcon, MoonIcon } from '../common/Icons';
import styles from './Navigation.module.css';
import clsx from 'clsx';

const navLinks = [
  { path: '/', name: 'Home' },
  { path: '/develop', name: 'Develop' },
  { path: '/research', name: 'Research' },
  { path: '/monitor', name: 'Monitor' },
];

export const Navigation: React.FC = () => {
  const location = useLocation();
  const { theme, toggleTheme, isMobileMenuOpen, toggleMobileMenu, user } = useAppStore();
  const [logoText, setLogoText] = useState('');
  const [showCursor, setShowCursor] = useState(true);
  const hasAnimated = sessionStorage.getItem('alphapulse_logo_animated');

  useEffect(() => {
    if (hasAnimated) {
      setLogoText('AlphaPulse');
      setShowCursor(false);
      return;
    }

    const text = 'AlphaPulse';
    let index = 0;

    const getTypingDelay = () => {
      const baseDelay = 80 + Math.random() * 70;
      if (Math.random() < 0.15) {
        return baseDelay + 100 + Math.random() * 200;
      }
      return baseDelay;
    };

    const typeNextChar = () => {
      if (index < text.length) {
        setLogoText(text.slice(0, index + 1));
        index++;
        setTimeout(typeNextChar, getTypingDelay());
      } else {
        setTimeout(() => {
          setShowCursor(false);
          sessionStorage.setItem('alphapulse_logo_animated', 'true');
        }, 3000);
      }
    };

    setTimeout(typeNextChar, 500);
  }, [hasAnimated]);

  // Close mobile menu on route change
  useEffect(() => {
    if (isMobileMenuOpen) {
      toggleMobileMenu();
    }
  }, [location.pathname]);

  return (
    <header className={styles.header}>
      <div className={styles.headerContent}>
        <button 
          className={styles.mobileMenuBtn} 
          onClick={toggleMobileMenu}
          aria-label="Toggle menu"
        >
          {isMobileMenuOpen ? <CloseIcon /> : <MenuIcon />}
        </button>
        
        <Link to="/" className={styles.logo}>
          <span className={styles.prompt}>{'>'}</span> 
          <span>{logoText}</span>
          {showCursor && <span className={styles.cursor}>_</span>}
        </Link>
        
        <nav className={clsx(styles.nav, { [styles.mobileOpen]: isMobileMenuOpen })}>
          <div className={styles.navLinks}>
            {navLinks.slice(1).map((link) => (
              <Link
                key={link.path}
                to={link.path}
                className={clsx(styles.navLink, {
                  [styles.active]: location.pathname === link.path
                })}
              >
                {link.name}
              </Link>
            ))}
          </div>
          
          <div className={styles.navActions}>
            <button 
              className={styles.themeToggle} 
              onClick={toggleTheme}
              aria-label="Toggle theme"
            >
              {theme === 'light' ? <SunIcon /> : <MoonIcon />}
            </button>
            
            <Link to="/login" className={styles.loginLink}>
              {user ? user.name : 'Login'}
            </Link>
          </div>
        </nav>
      </div>
    </header>
  );
};