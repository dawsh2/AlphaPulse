import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type Theme = 'light' | 'dark';

interface User {
  id: string;
  name: string;
  email?: string;
}

interface AppState {
  // Theme
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
  
  // User
  user: User | null;
  setUser: (user: User | null) => void;
  
  // Mobile menu
  isMobileMenuOpen: boolean;
  setMobileMenuOpen: (isOpen: boolean) => void;
  toggleMobileMenu: () => void;
  
  // Navigation
  currentPage: string;
  setCurrentPage: (page: string) => void;
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      // Theme
      theme: 'dark',
      setTheme: (theme) => {
        set({ theme });
        document.documentElement.setAttribute('data-theme', theme);
      },
      toggleTheme: () => {
        set((state) => {
          const newTheme = state.theme === 'light' ? 'dark' : 'light';
          document.documentElement.setAttribute('data-theme', newTheme);
          return { theme: newTheme };
        });
      },
      
      // User
      user: null,
      setUser: (user) => set({ user }),
      
      // Mobile menu
      isMobileMenuOpen: false,
      setMobileMenuOpen: (isOpen) => set({ isMobileMenuOpen: isOpen }),
      toggleMobileMenu: () => set((state) => ({ isMobileMenuOpen: !state.isMobileMenuOpen })),
      
      // Navigation
      currentPage: 'home',
      setCurrentPage: (page) => set({ currentPage: page }),
    }),
    {
      name: 'alphapulse-storage',
      partialize: (state) => ({ theme: state.theme, user: state.user }),
    }
  )
);

// Initialize theme on app load
if (typeof window !== 'undefined') {
  const storedTheme = localStorage.getItem('alphapulse-storage');
  if (storedTheme) {
    try {
      const { state } = JSON.parse(storedTheme);
      if (state?.theme) {
        document.documentElement.setAttribute('data-theme', state.theme);
      } else {
        // Default to dark theme if no stored preference
        document.documentElement.setAttribute('data-theme', 'dark');
      }
    } catch (error) {
      console.error('Failed to parse stored theme:', error);
      // Default to dark theme on error
      document.documentElement.setAttribute('data-theme', 'dark');
    }
  } else {
    // Default to dark theme if no stored data
    document.documentElement.setAttribute('data-theme', 'dark');
  }
}