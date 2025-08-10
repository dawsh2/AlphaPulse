#!/usr/bin/env python3
"""
Remove subtitle and add Strategy Builder functionality to explore.html
"""

# Read the file
with open('/Users/daws/alphapulse/ui/explore.html', 'r') as f:
    content = f.read()

# 1. Remove the subtitle line
import re
subtitle_pattern = r'<p class="catalogue-subtitle">Build, Test, and Deploy Trading Strategies</p>'
content = re.sub(subtitle_pattern, '', content)

# 2. Update the search container to include a + button
old_search = '''                <div class="catalogue-search">
                    <input 
                        type="text" 
                        placeholder="search strategies..." 
                        id="strategySearchInput"
                        class="search-input"
                    />
                </div>'''

new_search = '''                <div class="catalogue-search">
                    <div style="display: flex; gap: var(--space-2); align-items: center;">
                        <input 
                            type="text" 
                            placeholder="search strategies..." 
                            id="strategySearchInput"
                            class="search-input"
                            style="flex: 1;"
                        />
                        <button class="builder-button" onclick="openStrategyBuilder()" title="Create New Strategy">
                            +
                        </button>
                    </div>
                </div>'''

content = content.replace(old_search, new_search)

# 3. Add styles for the builder button and modal
builder_styles = '''
        /* Builder Button */
        .builder-button {
            width: 44px;
            height: 44px;
            border: 2px solid var(--color-border-primary);
            border-radius: var(--radius-md);
            background: var(--color-bg-primary);
            color: var(--color-text-primary);
            font-size: var(--font-size-xl);
            font-weight: var(--font-weight-bold);
            cursor: pointer;
            transition: all var(--transition-fast);
            display: flex;
            align-items: center;
            justify-content: center;
        }
        
        .builder-button:hover {
            background: var(--color-primary);
            color: white;
            border-color: var(--color-primary);
        }
        
        /* Strategy Builder Modal */
        .builder-modal {
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: rgba(0, 0, 0, 0.8);
            display: none;
            align-items: center;
            justify-content: center;
            z-index: 2000;
            padding: var(--space-4);
            overflow-y: auto;
        }
        
        .builder-modal.active {
            display: flex;
        }
        
        .builder-container {
            background: var(--color-bg-primary);
            border-radius: var(--radius-lg);
            width: 100%;
            max-width: 1200px;
            max-height: 90vh;
            overflow-y: auto;
            position: relative;
        }
        
        .builder-header {
            padding: var(--space-6);
            border-bottom: 1px solid var(--color-border-primary);
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        
        .builder-title {
            font-size: var(--font-size-2xl);
            font-weight: var(--font-weight-bold);
        }
        
        .builder-close {
            width: 36px;
            height: 36px;
            border: 2px solid var(--color-border-primary);
            border-radius: var(--radius-md);
            background: transparent;
            color: var(--color-text-primary);
            cursor: pointer;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: var(--font-size-xl);
            transition: all var(--transition-fast);
        }
        
        .builder-close:hover {
            background: var(--color-danger);
            color: white;
            border-color: var(--color-danger);
        }
        
        .builder-content {
            padding: var(--space-6);
        }
        
        /* Progress Steps */
        .progress-steps {
            display: flex;
            justify-content: space-between;
            margin-bottom: var(--space-8);
            padding: 0 var(--space-8);
        }
        
        .step {
            display: flex;
            flex-direction: column;
            align-items: center;
            text-align: center;
            flex: 1;
            position: relative;
        }
        
        .step:not(:last-child)::after {
            content: '';
            position: absolute;
            top: 20px;
            left: 60%;
            right: -40%;
            height: 2px;
            background: var(--color-border-primary);
        }
        
        .step.active .step-circle {
            background: var(--color-primary);
            color: white;
            border-color: var(--color-primary);
        }
        
        .step.completed .step-circle {
            background: var(--color-success);
            color: white;
            border-color: var(--color-success);
        }
        
        .step-circle {
            width: 40px;
            height: 40px;
            border-radius: 50%;
            border: 2px solid var(--color-border-primary);
            background: var(--color-bg-primary);
            display: flex;
            align-items: center;
            justify-content: center;
            font-weight: var(--font-weight-semibold);
            margin-bottom: var(--space-2);
        }
        
        .step-label {
            font-size: var(--font-size-sm);
            color: var(--color-text-secondary);
        }
        
        /* Builder Form */
        .builder-form {
            display: grid;
            gap: var(--space-6);
        }
        
        .form-section {
            background: var(--color-bg-secondary);
            border-radius: var(--radius-lg);
            padding: var(--space-6);
        }
        
        .form-section-title {
            font-size: var(--font-size-lg);
            font-weight: var(--font-weight-semibold);
            margin-bottom: var(--space-4);
        }
        
        .indicator-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
            gap: var(--space-3);
        }
        
        .indicator-option {
            padding: var(--space-3);
            border: 2px solid var(--color-border-primary);
            border-radius: var(--radius-md);
            text-align: center;
            cursor: pointer;
            transition: all var(--transition-fast);
        }
        
        .indicator-option:hover {
            border-color: var(--color-primary);
            background: var(--color-bg-primary);
        }
        
        .indicator-option.selected {
            background: var(--color-primary);
            color: white;
            border-color: var(--color-primary);
        }'''

# Insert the styles before </style>
content = content.replace('    </style>', builder_styles + '\n    </style>')

# 4. Add the Strategy Builder HTML at the end of body before scripts
builder_html = '''
    <!-- Strategy Builder Modal -->
    <div class="builder-modal" id="strategyBuilderModal">
        <div class="builder-container">
            <div class="builder-header">
                <h2 class="builder-title">Strategy Builder</h2>
                <button class="builder-close" onclick="closeStrategyBuilder()">×</button>
            </div>
            
            <div class="builder-content">
                <!-- Progress Steps -->
                <div class="progress-steps">
                    <div class="step active">
                        <div class="step-circle">1</div>
                        <div class="step-label">Template</div>
                    </div>
                    <div class="step">
                        <div class="step-circle">2</div>
                        <div class="step-label">Indicators</div>
                    </div>
                    <div class="step">
                        <div class="step-circle">3</div>
                        <div class="step-label">Parameters</div>
                    </div>
                    <div class="step">
                        <div class="step-circle">4</div>
                        <div class="step-label">Review</div>
                    </div>
                </div>
                
                <!-- Builder Form -->
                <div class="builder-form">
                    <div class="form-section">
                        <h3 class="form-section-title">Choose a Template</h3>
                        <div class="strategy-grid" style="margin-bottom: 0;">
                            <div class="strategy-card color-blue" onclick="selectTemplate('trend-following')" style="cursor: pointer;">
                                <h3 class="strategy-title">Trend Following</h3>
                                <p class="strategy-description">Moving averages & momentum</p>
                            </div>
                            <div class="strategy-card color-orange" onclick="selectTemplate('mean-reversion')" style="cursor: pointer;">
                                <h3 class="strategy-title">Mean Reversion</h3>
                                <p class="strategy-description">Buy low, sell high</p>
                            </div>
                            <div class="strategy-card color-green" onclick="selectTemplate('breakout')" style="cursor: pointer;">
                                <h3 class="strategy-title">Breakout</h3>
                                <p class="strategy-description">Price channel breaks</p>
                            </div>
                            <div class="strategy-card color-purple" onclick="selectTemplate('custom')" style="cursor: pointer;">
                                <h3 class="strategy-title">Custom</h3>
                                <p class="strategy-description">Build from scratch</p>
                            </div>
                        </div>
                    </div>
                    
                    <div class="form-section" style="display: none;" id="indicatorSection">
                        <h3 class="form-section-title">Select Indicators</h3>
                        <div class="indicator-grid">
                            <div class="indicator-option" onclick="toggleIndicator(this, 'rsi')">RSI</div>
                            <div class="indicator-option" onclick="toggleIndicator(this, 'macd')">MACD</div>
                            <div class="indicator-option" onclick="toggleIndicator(this, 'ema')">EMA</div>
                            <div class="indicator-option" onclick="toggleIndicator(this, 'sma')">SMA</div>
                            <div class="indicator-option" onclick="toggleIndicator(this, 'bb')">Bollinger</div>
                            <div class="indicator-option" onclick="toggleIndicator(this, 'atr')">ATR</div>
                        </div>
                    </div>
                    
                    <div style="display: flex; justify-content: space-between; margin-top: var(--space-6);">
                        <button class="btn" onclick="previousStep()">Previous</button>
                        <button class="btn btn-primary" onclick="nextStep()">Next</button>
                    </div>
                </div>
            </div>
        </div>
    </div>'''

# Insert before the closing body tag
content = content.replace('</body>', builder_html + '\n</body>')

# 5. Add JavaScript functions for the Strategy Builder
builder_js = '''
        // Strategy Builder Functions
        let currentBuilderStep = 1;
        let selectedTemplate = null;
        let selectedIndicators = [];
        
        function openStrategyBuilder() {
            document.getElementById('strategyBuilderModal').classList.add('active');
            document.body.style.overflow = 'hidden';
        }
        
        function closeStrategyBuilder() {
            document.getElementById('strategyBuilderModal').classList.remove('active');
            document.body.style.overflow = '';
            // Reset state
            currentBuilderStep = 1;
            selectedTemplate = null;
            selectedIndicators = [];
        }
        
        function selectTemplate(template) {
            selectedTemplate = template;
            // Highlight selected template
            document.querySelectorAll('.builder-form .strategy-card').forEach(card => {
                card.style.opacity = '0.5';
            });
            event.currentTarget.style.opacity = '1';
            
            // Auto advance to next step
            setTimeout(() => nextStep(), 300);
        }
        
        function toggleIndicator(element, indicator) {
            element.classList.toggle('selected');
            if (element.classList.contains('selected')) {
                selectedIndicators.push(indicator);
            } else {
                selectedIndicators = selectedIndicators.filter(i => i !== indicator);
            }
        }
        
        function nextStep() {
            if (currentBuilderStep < 4) {
                currentBuilderStep++;
                updateBuilderSteps();
            }
        }
        
        function previousStep() {
            if (currentBuilderStep > 1) {
                currentBuilderStep--;
                updateBuilderSteps();
            }
        }
        
        function updateBuilderSteps() {
            // Update step indicators
            document.querySelectorAll('.progress-steps .step').forEach((step, index) => {
                step.classList.remove('active', 'completed');
                if (index + 1 < currentBuilderStep) {
                    step.classList.add('completed');
                    step.querySelector('.step-circle').textContent = '✓';
                } else if (index + 1 === currentBuilderStep) {
                    step.classList.add('active');
                } else {
                    step.querySelector('.step-circle').textContent = index + 1;
                }
            });
            
            // Show/hide sections based on step
            if (currentBuilderStep === 2) {
                document.getElementById('indicatorSection').style.display = 'block';
            }
        }
        
        // Close modal on backdrop click
        document.addEventListener('click', function(e) {
            if (e.target.classList.contains('builder-modal')) {
                closeStrategyBuilder();
            }
        });'''

# Insert the JavaScript before the closing script tag
content = content.replace('    </script>', builder_js + '\n    </script>')

# Write the updated file
with open('/Users/daws/alphapulse/ui/explore.html', 'w') as f:
    f.write(content)

print("✅ Removed subtitle")
print("✅ Added + button to search bar")
print("✅ Integrated Strategy Builder modal")
print("✅ Added progress steps and template selection")
print("\n✨ Strategy Builder can now be opened with the + button!")