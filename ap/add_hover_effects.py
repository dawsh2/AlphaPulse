#!/usr/bin/env python3
"""
Add hover effects CSS for strategy cards
"""

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# Add hover CSS styles
hover_styles = '''
        /* Strategy Card Hover Effects */
        .strategy-card:hover {
            transform: translate(2px, -2px) !important;
            box-shadow: -5px 7px currentColor !important;
        }
        
        .strategy-card:active {
            transform: translate(-1px, 2px) !important;
            box-shadow: -2px 3px currentColor !important;
        }
        
        /* Color-specific hover states */
        .color-blue:hover {
            background: #1a1a1a !important;
            color: #89CDF1 !important;
            border-color: #5BA7D1 !important;
        }
        
        .color-blue:hover .strategy-title,
        .color-blue:hover .strategy-description,
        .color-blue:hover .tag {
            color: #89CDF1 !important;
        }
        
        .color-orange:hover {
            background: #1a1a1a !important;
            color: #FF9500 !important;
            border-color: #CC7700 !important;
        }
        
        .color-orange:hover .strategy-title,
        .color-orange:hover .strategy-description,
        .color-orange:hover .tag {
            color: #FF9500 !important;
        }
        
        .color-green:hover {
            background: #1a1a1a !important;
            color: #4CAF50 !important;
            border-color: #388E3C !important;
        }
        
        .color-green:hover .strategy-title,
        .color-green:hover .strategy-description,
        .color-green:hover .tag {
            color: #4CAF50 !important;
        }
        
        .color-purple:hover {
            background: #ffffff !important;
            color: #9C27B0 !important;
            border-color: #7B1FA2 !important;
        }
        
        .color-purple:hover .strategy-title,
        .color-purple:hover .strategy-description,
        .color-purple:hover .tag {
            color: #9C27B0 !important;
        }
        
        .color-red:hover {
            background: #ffffff !important;
            color: #F44336 !important;
            border-color: #D32F2F !important;
        }
        
        .color-red:hover .strategy-title,
        .color-red:hover .strategy-description,
        .color-red:hover .tag {
            color: #F44336 !important;
        }
        
        .color-teal:hover {
            background: #ffffff !important;
            color: #009688 !important;
            border-color: #00695C !important;
        }
        
        .color-teal:hover .strategy-title,
        .color-teal:hover .strategy-description,
        .color-teal:hover .tag {
            color: #009688 !important;
        }
        
        .color-indigo:hover {
            background: #ffffff !important;
            color: #3F51B5 !important;
            border-color: #303F9F !important;
        }
        
        .color-indigo:hover .strategy-title,
        .color-indigo:hover .strategy-description,
        .color-indigo:hover .tag {
            color: #3F51B5 !important;
        }
        
        .color-pink:hover {
            background: #ffffff !important;
            color: #E91E63 !important;
            border-color: #C2185B !important;
        }
        
        .color-pink:hover .strategy-title,
        .color-pink:hover .strategy-description,
        .color-pink:hover .tag {
            color: #E91E63 !important;
        }
        
        .color-cyan:hover {
            background: #1a1a1a !important;
            color: #00BCD4 !important;
            border-color: #0097A7 !important;
        }
        
        .color-cyan:hover .strategy-title,
        .color-cyan:hover .strategy-description,
        .color-cyan:hover .tag {
            color: #00BCD4 !important;
        }
'''

# Insert the hover styles before the closing style tag
content = content.replace('    </style>', hover_styles + '\n    </style>')

# Write the updated content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.write(content)

print("✅ Added hover effects for strategy cards")
print("✨ Strategy cards now have proper AlphaPulse button styling with hover effects")