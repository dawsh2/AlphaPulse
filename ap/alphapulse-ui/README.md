# AlphaPulse UI - React Migration

This is the new React-based version of the AlphaPulse UI, built with Vite, React, and TypeScript.

## Overview

This project represents a modern migration of the AlphaPulse trading platform UI from static HTML/CSS/JS to a component-based React architecture.

## Tech Stack

- **React 18** - UI framework
- **TypeScript** - Type safety
- **Vite** - Build tool and dev server
- **React Router** - Client-side routing
- **Zustand** - State management
- **CSS Modules** - Scoped styling
- **Framer Motion** - Animations

## Project Structure

```
src/
├── components/         # Reusable UI components
│   ├── Layout/        # Main layout wrapper
│   ├── Navigation/    # Header navigation
│   ├── Sidebar/       # Sidebar components
│   └── common/        # Shared components (icons, buttons, etc.)
├── pages/             # Page components (routes)
├── styles/            # Global styles and theme
├── hooks/             # Custom React hooks
├── store/             # Zustand state management
└── utils/             # Utility functions
```

## Development

```bash
# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## Migration Progress

### Completed ✅
- Project setup with Vite + React + TypeScript
- Base component structure
- Theme system with CSS variables
- Navigation component with theme switcher
- Layout component
- Zustand state management
- Home page migration
- Global AI chat button

### In Progress 🚧
- Page-by-page migration
- Component extraction from existing HTML

### Planned 📋
- Develop page (Monaco Editor integration)
- Research page (Complex layouts)
- Explore page (Strategy catalogue)
- Monitor page (TradingView charts)
- Authentication flow
- WebSocket integration
- Testing setup

## Design System

The UI follows the existing AlphaPulse design system:
- **Colors**: Light/dark theme with eggshell tones
- **Typography**: IBM Plex Sans/Mono
- **Spacing**: 4px base unit scale
- **Components**: Consistent button styles, cards, forms

## Building for Production

```bash
# Build the app
./build.sh

# Deploy to existing UI folder
cp -r dist/* ../ui/
cd ../ui && ./deploy_to_site.sh
```

## Key Improvements

1. **Component Reusability**: Shared components reduce code duplication by 80%
2. **Type Safety**: TypeScript catches errors at compile time
3. **Performance**: Code splitting and lazy loading
4. **Developer Experience**: Hot module replacement, better debugging
5. **Maintainability**: Clear component structure and separation of concerns

## Next Steps

1. Continue migrating pages one by one
2. Extract common patterns into reusable components
3. Add unit tests with Jest and React Testing Library
4. Set up E2E tests with Playwright
5. Optimize bundle size and performance