#!/usr/bin/env python3
"""
Add the full Strategy Builder content to explore.html
"""

# Read the file
with open('/Users/daws/alphapulse/ui/explore.html', 'r') as f:
    content = f.read()

# Replace the empty modal with the full builder content
old_modal = '<div class="builder-modal" id="strategyBuilderModal">'
new_modal = '''<div class="builder-modal" id="strategyBuilderModal">
        <div class="builder-container">
            <div class="builder-header">
                <h2 class="builder-title">Strategy Builder</h2>
                <button class="builder-close" onclick="closeStrategyBuilder()">×</button>
            </div>
            
            <div class="builder-content">
                <!-- Progress Steps -->
                <div class="progress-steps">
                    <div class="step completed" data-step="1">
                        <div class="step-circle">✓</div>
                        <div class="step-label">Template</div>
                    </div>
                    <div class="step active" data-step="2">
                        <div class="step-circle">2</div>
                        <div class="step-label">Indicators</div>
                    </div>
                    <div class="step" data-step="3">
                        <div class="step-circle">3</div>
                        <div class="step-label">Parameters</div>
                    </div>
                    <div class="step" data-step="4">
                        <div class="step-circle">4</div>
                        <div class="step-label">Universe</div>
                    </div>
                    <div class="step" data-step="5">
                        <div class="step-circle">5</div>
                        <div class="step-label">Backtest</div>
                    </div>
                    <div class="step" data-step="6">
                        <div class="step-circle">6</div>
                        <div class="step-label">Results</div>
                    </div>
                </div>
                
                <!-- Main Content Grid -->
                <div style="display: grid; grid-template-columns: 300px 1fr; gap: var(--space-8); margin-top: var(--space-8);">
                    <!-- Sidebar -->
                    <aside style="background: var(--color-bg-secondary); border: 2px solid var(--color-border-primary); border-radius: var(--radius-lg); padding: var(--space-4);">
                        <h3 style="font-family: var(--font-family-mono); font-size: var(--font-size-lg); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-4);">Indicator Library</h3>
                        
                        <div style="position: relative; margin-bottom: var(--space-4);">
                            <input 
                                type="text" 
                                placeholder="Type indicator name..." 
                                id="indicator-search"
                                style="width: 100%; padding: var(--space-3); font-family: var(--font-family-mono); font-size: var(--font-size-sm); background: var(--color-bg-primary); border: 2px solid var(--color-border-primary); border-radius: var(--radius-md); color: var(--color-text-primary);"
                                oninput="filterIndicators(this.value)"
                            />
                            <div id="search-results" style="display: none; position: absolute; top: 100%; left: 0; right: 0; background: var(--color-bg-primary); border: 2px solid var(--color-border-primary); border-radius: var(--radius-md); max-height: 300px; overflow-y: auto; z-index: 10; margin-top: var(--space-1);"></div>
                        </div>
                        
                        <div style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); color: var(--color-text-tertiary); font-style: italic;">
                            Try: 'rsi', 'bollinger', 'macd', 'ema'
                        </div>
                    </aside>
                    
                    <!-- Main Panel -->
                    <main style="background: var(--color-bg-secondary); border: 2px solid var(--color-border-primary); border-radius: var(--radius-lg); padding: var(--space-6);">
                        <h2 style="font-family: var(--font-family-mono); font-size: var(--font-size-xl); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-6);">Build Your Strategy Logic</h2>
                        
                        <!-- Strategy Canvas -->
                        <div id="strategy-canvas" style="min-height: 300px; background: var(--color-bg-primary); border: 2px dashed var(--color-border-primary); border-radius: var(--radius-lg); display: flex; align-items: center; justify-content: center; margin-bottom: var(--space-6); padding: var(--space-4);">
                            <div class="canvas-placeholder" style="text-align: center; color: var(--color-text-secondary); font-family: var(--font-family-mono);">
                                <div style="font-size: var(--font-size-lg); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-2);">Drag indicators here</div>
                                <div style="font-size: var(--font-size-sm);">Start typing in the search box to find indicators</div>
                            </div>
                        </div>
                        
                        <!-- Logic Builder -->
                        <div style="background: var(--color-bg-primary); border: 2px solid var(--color-border-primary); border-radius: var(--radius-lg); padding: var(--space-4); margin-bottom: var(--space-4);">
                            <h3 style="font-family: var(--font-family-mono); font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-3);">Signal Combination Logic</h3>
                            <div style="display: flex; gap: var(--space-2); flex-wrap: wrap;">
                                <div class="logic-option active" onclick="selectLogic(this, 'AND')" style="padding: var(--space-2) var(--space-3); background: var(--color-primary); color: white; border: 2px solid var(--color-primary); border-radius: var(--radius-md); font-family: var(--font-family-mono); font-size: var(--font-size-sm); cursor: pointer;">ALL signals must be true (AND)</div>
                                <div class="logic-option" onclick="selectLogic(this, 'OR')" style="padding: var(--space-2) var(--space-3); background: var(--color-bg-secondary); border: 2px solid var(--color-border-primary); border-radius: var(--radius-md); font-family: var(--font-family-mono); font-size: var(--font-size-sm); cursor: pointer;">ANY signal can be true (OR)</div>
                                <div class="logic-option" onclick="selectLogic(this, 'CUSTOM')" style="padding: var(--space-2) var(--space-3); background: var(--color-bg-secondary); border: 2px solid var(--color-border-primary); border-radius: var(--radius-md); font-family: var(--font-family-mono); font-size: var(--font-size-sm); cursor: pointer;">Custom logic builder</div>
                            </div>
                        </div>
                        
                        <!-- Actions -->
                        <div style="display: flex; justify-content: space-between; align-items: center;">
                            <div style="display: flex; gap: var(--space-2);">
                                <button class="btn" onclick="clearCanvas()">Clear All</button>
                                <button class="btn" onclick="saveAsTemplate()">Save as Template</button>
                            </div>
                            <div style="display: flex; gap: var(--space-2);">
                                <button class="btn" onclick="previousBuilderStep()">← Back</button>
                                <button class="btn btn-primary" onclick="nextBuilderStep()">Parameters →</button>
                            </div>
                        </div>
                    </main>
                </div>
            </div>
        </div>'''

content = content.replace(old_modal, new_modal)

# Write back
with open('/Users/daws/alphapulse/ui/explore.html', 'w') as f:
    f.write(content)

print("✅ Added full Strategy Builder interface")
print("✅ Includes indicator search, strategy canvas, and logic builder")
print("✨ The + button now opens the complete Strategy Builder!")