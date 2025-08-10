#!/usr/bin/env python3
"""
Restore research.html to show notebook view
"""

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# Check if we need to restore the notebook
if 'catalogue-container' in content:
    print("Found catalogue container, restoring notebook view...")
    
    # The original notebook HTML
    notebook_html = '''            <!-- Notebook Container -->
            <div class="notebook-container">
                <div class="notebook-header">
                    <input type="text" class="notebook-title-input" value="Strategy Analysis - Jan 2024" />
                    <div class="notebook-actions">
                        <button class="btn btn-ghost btn-sm" onclick="runAllCells()">Run All</button>
                        <button class="btn btn-ghost btn-sm" onclick="clearOutputs()">Clear</button>
                        <button class="btn btn-ghost btn-sm" onclick="exportNotebook()">Export</button>
                        <button class="btn btn-primary btn-sm" onclick="saveNotebook()">Save</button>
                    </div>
                </div>
                
                <div class="notebook-content" id="notebookContent">
                    <!-- Markdown Cell -->
                    <div class="cell markdown-cell">
                        <div class="cell-toolbar">
                            <button class="cell-btn" onclick="runCell(this)">▶</button>
                            <button class="cell-btn" onclick="deleteCell(this)">✕</button>
                        </div>
                        <div class="markdown-preview">
                            <h1>Strategy Analysis</h1>
                            <p>Analyzing momentum strategies from the latest config run. The backend automatically generated signals for any missing parameter combinations.</p>
                        </div>
                    </div>
                    
                    <!-- Code Cell 1: Load Data -->
                    <div class="cell code-cell active" id="cell-1">
                        <div class="cell-toolbar">
                            <button class="cell-btn" onclick="runCell(this)">▶</button>
                            <button class="cell-btn" onclick="deleteCell(this)">✕</button>
                        </div>
                        <div class="cell-editor">
                            <pre><code class="language-python"># Load strategy signals from registry
from datetime import datetime
import pandas as pd
import numpy as np

# Get signals for the selected date range
signals = load_signals(
    strategy='momentum',
    start_date='2024-01-01',
    end_date='2024-01-31'
)

print(f"Loaded {len(signals)} signals")
signals.head()</code></pre>
                        </div>
                        <div class="cell-output">
                            <pre>Loaded 523 signals
   timestamp  symbol  signal  position  pnl
0  2024-01-02  AAPL   1.0     100      0.0
1  2024-01-02  MSFT   0.0     0        0.0
2  2024-01-02  GOOGL  1.0     50       0.0</pre>
                        </div>
                    </div>
                    
                    <!-- Code Cell 2: Performance Metrics -->
                    <div class="cell code-cell" id="cell-2">
                        <div class="cell-toolbar">
                            <button class="cell-btn" onclick="runCell(this)">▶</button>
                            <button class="cell-btn" onclick="deleteCell(this)">✕</button>
                        </div>
                        <div class="cell-editor">
                            <pre><code class="language-python"># Calculate performance metrics
from alphapulse.metrics import calculate_metrics

metrics = calculate_metrics(signals)
print("Strategy Performance:")
for metric, value in metrics.items():
    print(f"{metric}: {value:.2f}")</code></pre>
                        </div>
                        <div class="cell-output"></div>
                    </div>
                    
                    <!-- Add New Cell Button -->
                    <div class="add-cell-container">
                        <button class="add-cell-btn" onclick="addCell('code')">+ Code</button>
                        <button class="add-cell-btn" onclick="addCell('markdown')">+ Text</button>
                    </div>
                </div>
            </div>'''
    
    # Replace catalogue container with notebook container
    import re
    catalogue_pattern = r'<!-- Catalogue Container -->.*?</div>\s*(?=</div>\s*</body>|<!-- Strategy Preview Overlay -->|$)'
    content = re.sub(catalogue_pattern, notebook_html, content, flags=re.DOTALL)
    
    # Write back
    with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
        f.write(content)
    
    print("✅ Restored research.html to notebook view")
else:
    print("ℹ️  Research.html already shows notebook view")

# Also check if selectedStrategy handling exists, add if not
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

if 'selectedStrategy' not in content:
    # Add script to handle selectedStrategy from explore page
    strategy_handler = '''
        // Handle selected strategy from explore page
        document.addEventListener('DOMContentLoaded', function() {
            const selectedStrategy = sessionStorage.getItem('selectedStrategy');
            if (selectedStrategy) {
                // Clear the selection
                sessionStorage.removeItem('selectedStrategy');
                
                // Load the strategy template
                console.log('Loading strategy:', selectedStrategy);
                
                // Update the notebook title
                const titleInput = document.querySelector('.notebook-title-input');
                if (titleInput) {
                    const strategyNames = {
                        'ema-cross': 'EMA Cross Strategy Analysis',
                        'mean-reversion': 'Mean Reversion Strategy Analysis',
                        'momentum': 'Momentum Strategy Analysis',
                        'pairs-trading': 'Pairs Trading Analysis',
                        'custom': 'Custom Strategy Development'
                    };
                    titleInput.value = strategyNames[selectedStrategy] || 'Strategy Analysis';
                }
                
                // You could also load a specific template or notebook here
                // loadStrategyNotebook(selectedStrategy);
            }
        });
'''
    
    # Insert before closing script tag
    content = content.replace('    </script>', strategy_handler + '\n    </script>')
    
    with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
        f.write(content)
    
    print("✅ Added selectedStrategy handler to research.html")