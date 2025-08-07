#!/usr/bin/env python3
"""
Add CSS variables and base styles to explore.html
"""

# Read the file
with open('/Users/daws/alphapulse/ui/explore.html', 'r') as f:
    content = f.read()

# CSS variables and base styles
css_variables = '''        :root {
            /* Typography */
            --font-family-sans: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            --font-family-mono: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, 'Courier New', monospace;
            
            /* Font Sizes */
            --font-size-xs: 0.75rem;
            --font-size-sm: 0.875rem;
            --font-size-base: 1rem;
            --font-size-lg: 1.125rem;
            --font-size-xl: 1.25rem;
            --font-size-2xl: 1.5rem;
            --font-size-3xl: 1.875rem;
            --font-size-4xl: 2.25rem;
            
            /* Font Weights */
            --font-weight-normal: 400;
            --font-weight-medium: 500;
            --font-weight-semibold: 600;
            --font-weight-bold: 700;
            
            /* Spacing */
            --space-1: 0.25rem;
            --space-2: 0.5rem;
            --space-3: 0.75rem;
            --space-4: 1rem;
            --space-6: 1.5rem;
            --space-8: 2rem;
            --space-12: 3rem;
            
            /* Colors - Light Theme */
            --color-bg-primary: #ffffff;
            --color-bg-secondary: #f6f8fa;
            --color-bg-tertiary: #f0f3f6;
            --color-text-primary: #1f2937;
            --color-text-secondary: #4b5563;
            --color-text-tertiary: #6b7280;
            --color-border-primary: #e5e7eb;
            --color-border-secondary: #d1d5db;
            
            /* Accent Colors */
            --color-primary: #0969da;
            --color-secondary: #8250df;
            --color-success: #1a7f37;
            --color-warning: #9a6700;
            --color-danger: #cf222e;
            
            /* Misc */
            --radius-sm: 0.25rem;
            --radius-md: 0.375rem;
            --radius-lg: 0.5rem;
            --radius-xl: 0.75rem;
            --header-height: 65px;
            --transition-fast: 150ms cubic-bezier(0.4, 0, 0.2, 1);
            --transition-base: 200ms cubic-bezier(0.4, 0, 0.2, 1);
        }
        
        /* Dark Theme */
        [data-theme="dark"] {
            --color-bg-primary: #0d1117;
            --color-bg-secondary: #161b22;
            --color-bg-tertiary: #21262d;
            --color-text-primary: #f0f6fc;
            --color-text-secondary: #8b949e;
            --color-text-tertiary: #6e7681;
            --color-border-primary: #30363d;
            --color-border-secondary: #21262d;
        }
        
        /* Base Styles */
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: var(--font-family-sans);
            font-size: var(--font-size-base);
            line-height: 1.5;
            color: var(--color-text-primary);
            background-color: var(--color-bg-primary);
            min-height: 100vh;
        }
        
        /* Header Styles */
        .header {
            height: var(--header-height);
            background: var(--color-bg-primary);
            border-bottom: 1px solid var(--color-border-primary);
            position: sticky;
            top: 0;
            z-index: 100;
        }
        
        .nav-container {
            max-width: 1280px;
            margin: 0 auto;
            height: 100%;
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 0 var(--space-6);
        }
        
        .logo-link {
            display: flex;
            align-items: center;
            gap: var(--space-2);
            text-decoration: none;
            color: var(--color-text-primary);
            font-weight: var(--font-weight-semibold);
            font-size: var(--font-size-lg);
        }
        
        .logo-text {
            font-family: var(--font-family-mono);
        }
        
        .nav-links {
            display: flex;
            gap: var(--space-6);
        }
        
        .nav-link {
            color: var(--color-text-secondary);
            text-decoration: none;
            font-weight: var(--font-weight-medium);
            transition: color var(--transition-fast);
            position: relative;
        }
        
        .nav-link:hover {
            color: var(--color-text-primary);
        }
        
        .nav-link.active {
            color: var(--color-text-primary);
        }
        
        .nav-link.active::after {
            content: '';
            position: absolute;
            bottom: -22px;
            left: 0;
            right: 0;
            height: 3px;
            background: var(--color-primary);
            border-radius: 3px 3px 0 0;
        }
        
        /* Button Styles */
        .btn {
            display: inline-flex;
            align-items: center;
            gap: var(--space-2);
            padding: var(--space-2) var(--space-4);
            font-family: inherit;
            font-size: var(--font-size-sm);
            font-weight: var(--font-weight-medium);
            border-radius: var(--radius-md);
            border: 1px solid var(--color-border-primary);
            background: var(--color-bg-primary);
            color: var(--color-text-primary);
            cursor: pointer;
            transition: all var(--transition-fast);
            text-decoration: none;
        }
        
        .btn:hover {
            background: var(--color-bg-secondary);
            border-color: var(--color-border-secondary);
        }
        
        .btn-primary {
            background: var(--color-primary);
            color: white;
            border-color: var(--color-primary);
        }
        
        .btn-primary:hover {
            background: #0860ca;
            border-color: #0860ca;
        }
        
'''

# Insert after <style> tag
content = content.replace('<style>', '<style>\n' + css_variables)

# Write the updated content
with open('/Users/daws/alphapulse/ui/explore.html', 'w') as f:
    f.write(content)

print("✅ Added CSS variables and base styles")
print("✨ Explore page should now have consistent styling!")