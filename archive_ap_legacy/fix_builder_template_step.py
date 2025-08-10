#!/usr/bin/env python3
"""
Fix Strategy Builder to show Template selection as first step
"""

# Read the file
with open('/Users/daws/alphapulse/ui/explore.html', 'r') as f:
    content = f.read()

# Find the main panel content and replace it with template selection for step 1
import re

# Find the main panel section
main_panel_pattern = r'<!-- Main Panel -->.*?</main>'

new_main_panel = '''<!-- Main Panel -->
                    <main style="background: var(--color-bg-secondary); border: 2px solid var(--color-border-primary); border-radius: var(--radius-lg); padding: var(--space-6);">
                        <!-- Step 1: Template Selection -->
                        <div id="step-template" class="step-panel active">
                            <h2 style="font-family: var(--font-family-mono); font-size: var(--font-size-xl); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-6);">Choose a Strategy Template</h2>
                            
                            <div class="strategy-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--space-4); margin-bottom: var(--space-6);">
                                <div class="strategy-card color-blue" onclick="selectBuilderTemplate('trend-following')" style="cursor: pointer; aspect-ratio: 1; padding: var(--space-4); display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center;">
                                    <h3 class="strategy-title">Trend Following</h3>
                                    <p class="strategy-description">EMA crosses & momentum</p>
                                </div>
                                <div class="strategy-card color-orange" onclick="selectBuilderTemplate('mean-reversion')" style="cursor: pointer; aspect-ratio: 1; padding: var(--space-4); display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center;">
                                    <h3 class="strategy-title">Mean Reversion</h3>
                                    <p class="strategy-description">RSI & Bollinger Bands</p>
                                </div>
                                <div class="strategy-card color-green" onclick="selectBuilderTemplate('breakout')" style="cursor: pointer; aspect-ratio: 1; padding: var(--space-4); display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center;">
                                    <h3 class="strategy-title">Breakout</h3>
                                    <p class="strategy-description">Channel & volatility</p>
                                </div>
                                <div class="strategy-card color-purple" onclick="selectBuilderTemplate('custom')" style="cursor: pointer; aspect-ratio: 1; padding: var(--space-4); display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center;">
                                    <h3 class="strategy-title">Custom</h3>
                                    <p class="strategy-description">Build from scratch</p>
                                </div>
                            </div>
                            
                            <div style="display: flex; justify-content: flex-end;">
                                <button class="btn" onclick="closeStrategyBuilder()">Cancel</button>
                            </div>
                        </div>
                        
                        <!-- Step 2: Indicators -->
                        <div id="step-indicators" class="step-panel" style="display: none;">
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
                                    <button class="btn" onclick="goToBuilderStep(1)">← Back</button>
                                    <button class="btn btn-primary" onclick="goToBuilderStep(3)">Parameters →</button>
                                </div>
                            </div>
                        </div>
                    </main>'''

# Replace the main panel
content = re.sub(main_panel_pattern, new_main_panel, content, flags=re.DOTALL)

# Update the step initialization to start at 1
content = content.replace('currentBuilderStep = 2;', 'currentBuilderStep = 1;')

# Update the initial step state
content = content.replace(
    '<div class="step completed" data-step="1">',
    '<div class="step active" data-step="1">'
)
content = content.replace(
    '<div class="step-circle">✓</div>\n                        <div class="step-label">Template</div>',
    '<div class="step-circle">1</div>\n                        <div class="step-label">Template</div>'
)
content = content.replace(
    '<div class="step active" data-step="2">',
    '<div class="step" data-step="2">'
)

# Add the new JavaScript functions
new_js = '''
        function selectBuilderTemplate(template) {
            console.log('Selected template:', template);
            // Store the selected template
            window.selectedTemplate = template;
            
            // If custom, skip to indicators, otherwise load template
            if (template !== 'custom') {
                // Pre-populate some indicators based on template
                const templateIndicators = {
                    'trend-following': ['EMA', 'SMA', 'MACD'],
                    'mean-reversion': ['RSI', 'Bollinger Bands'],
                    'breakout': ['ATR', 'Bollinger Bands', 'Volume']
                };
                window.templateIndicators = templateIndicators[template] || [];
            }
            
            // Move to next step
            goToBuilderStep(2);
        }
        
        function goToBuilderStep(step) {
            // Hide all panels
            document.querySelectorAll('.step-panel').forEach(panel => {
                panel.style.display = 'none';
            });
            
            // Show the target panel
            const panelIds = {
                1: 'step-template',
                2: 'step-indicators',
                3: 'step-parameters',
                4: 'step-universe',
                5: 'step-backtest',
                6: 'step-results'
            };
            
            const targetPanel = document.getElementById(panelIds[step]);
            if (targetPanel) {
                targetPanel.style.display = 'block';
            }
            
            // Update progress steps
            document.querySelectorAll('.progress-steps .step').forEach((stepEl, index) => {
                const stepNum = index + 1;
                stepEl.classList.remove('active', 'completed');
                
                if (stepNum < step) {
                    stepEl.classList.add('completed');
                    stepEl.querySelector('.step-circle').textContent = '✓';
                } else if (stepNum === step) {
                    stepEl.classList.add('active');
                    stepEl.querySelector('.step-circle').textContent = stepNum;
                } else {
                    stepEl.querySelector('.step-circle').textContent = stepNum;
                }
            });
            
            currentBuilderStep = step;
            
            // If moving to indicators step and we have template indicators, add them
            if (step === 2 && window.templateIndicators) {
                setTimeout(() => {
                    window.templateIndicators.forEach(ind => {
                        addIndicatorToBuilder(ind);
                    });
                    window.templateIndicators = null;
                }, 100);
            }
        }'''

# Insert the new functions before the closing script tag
content = content.replace('    </script>', new_js + '\n    </script>')

# Write back
with open('/Users/daws/alphapulse/ui/explore.html', 'w') as f:
    f.write(content)

print("✅ Fixed Strategy Builder to show Template as first step")
print("✅ Added template selection grid")
print("✅ Added step navigation functions")
print("✅ Templates will pre-populate relevant indicators")