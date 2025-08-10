#!/usr/bin/env python3
"""
Replace the notebook container with catalogue view
"""

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# Define the catalogue content to replace the notebook
catalogue_html = '''            <!-- Catalogue Container -->
            <div class="catalogue-container" style="flex: 1; overflow-y: auto; padding: var(--space-6);">
                <div class="catalogue-header" style="text-align: center; margin-bottom: var(--space-8);">
                    <h1 style="font-size: var(--font-size-3xl); font-weight: var(--font-weight-bold); margin-bottom: var(--space-2);">Strategy Catalogue</h1>
                    <p style="color: var(--color-text-secondary); font-size: var(--font-size-lg);">Build, Test, and Deploy Trading Strategies</p>
                </div>
                
                <div class="catalogue-search" style="max-width: 500px; margin: 0 auto var(--space-6) auto;">
                    <input 
                        type="text" 
                        placeholder="search strategies..." 
                        id="strategySearchInput"
                        onkeyup="searchStrategies()"
                        style="width: 100%; padding: var(--space-3) var(--space-4); font-family: var(--font-family-mono); font-size: var(--font-size-base); background: var(--color-bg-primary); border: 3px solid var(--color-text-primary); border-radius: var(--radius-md); color: var(--color-text-primary); box-shadow: -3px 5px var(--color-text-primary); transition: all var(--transition-fast);"
                        onfocus="this.style.transform='translate(2px, -2px)'; this.style.boxShadow='-5px 7px var(--color-text-primary)';"
                        onblur="this.style.transform=''; this.style.boxShadow='-3px 5px var(--color-text-primary)';"
                    />
                </div>
                
                <section class="strategy-section" style="margin-bottom: var(--space-8);">
                    <h2 class="section-title" style="font-family: var(--font-family-mono); font-size: var(--font-size-xl); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-4); text-align: center; color: var(--color-text-secondary);">core strategies</h2>
                    <div class="strategy-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: var(--space-6);">
                        
                        <button class="strategy-card color-blue" onclick="selectStrategy('ema-cross')" style="font-family: var(--font-family-sans); background: #89CDF1; border: 3px solid #5BA7D1; color: #1a1a1a; border-radius: var(--radius-lg); box-shadow: -3px 5px #5BA7D1; cursor: pointer; transition: all var(--transition-fast); padding: var(--space-4); text-align: left; position: relative; margin-bottom: 5px; margin-right: 5px; min-height: 140px; display: flex; flex-direction: column; width: 100%;">
                            <h3 class="strategy-title" style="font-family: var(--font-family-mono); font-size: var(--font-size-lg); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-2); line-height: 1.2;">EMA Cross</h3>
                            <p class="strategy-description" style="font-size: var(--font-size-sm); color: #2a2a2a; line-height: 1.4; margin-bottom: var(--space-3); flex-grow: 1;">Classic trend-following strategy using exponential moving average crossovers.</p>
                            <div class="strategy-tags" style="display: flex; flex-wrap: wrap; gap: var(--space-1); margin-top: auto;">
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.3); color: #1a1a1a; border: 1px solid #5BA7D1; border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">trend following</span>
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.3); color: #1a1a1a; border: 1px solid #5BA7D1; border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">beginner</span>
                            </div>
                        </button>
                        
                        <button class="strategy-card color-orange" onclick="selectStrategy('mean-reversion')" style="font-family: var(--font-family-sans); background: #FF9500; border: 3px solid #CC7700; color: #1a1a1a; border-radius: var(--radius-lg); box-shadow: -3px 5px #CC7700; cursor: pointer; transition: all var(--transition-fast); padding: var(--space-4); text-align: left; position: relative; margin-bottom: 5px; margin-right: 5px; min-height: 140px; display: flex; flex-direction: column; width: 100%;">
                            <h3 class="strategy-title" style="font-family: var(--font-family-mono); font-size: var(--font-size-lg); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-2); line-height: 1.2;">Mean Reversion</h3>
                            <p class="strategy-description" style="font-size: var(--font-size-sm); color: #2a2a2a; line-height: 1.4; margin-bottom: var(--space-3); flex-grow: 1;">Statistical arbitrage strategy that profits from temporary price dislocations.</p>
                            <div class="strategy-tags" style="display: flex; flex-wrap: wrap; gap: var(--space-1); margin-top: auto;">
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.3); color: #1a1a1a; border: 1px solid #CC7700; border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">statistical arb</span>
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.3); color: #1a1a1a; border: 1px solid #CC7700; border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">intermediate</span>
                            </div>
                        </button>
                        
                        <button class="strategy-card color-green" onclick="selectStrategy('momentum')" style="font-family: var(--font-family-sans); background: #4CAF50; border: 3px solid #388E3C; color: #1a1a1a; border-radius: var(--radius-lg); box-shadow: -3px 5px #388E3C; cursor: pointer; transition: all var(--transition-fast); padding: var(--space-4); text-align: left; position: relative; margin-bottom: 5px; margin-right: 5px; min-height: 140px; display: flex; flex-direction: column; width: 100%;">
                            <h3 class="strategy-title" style="font-family: var(--font-family-mono); font-size: var(--font-size-lg); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-2); line-height: 1.2;">Momentum</h3>
                            <p class="strategy-description" style="font-size: var(--font-size-sm); color: #2a2a2a; line-height: 1.4; margin-bottom: var(--space-3); flex-grow: 1;">Trend-following strategies that capitalize on price continuations.</p>
                            <div class="strategy-tags" style="display: flex; flex-wrap: wrap; gap: var(--space-1); margin-top: auto;">
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.3); color: #1a1a1a; border: 1px solid #388E3C; border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">trend</span>
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.3); color: #1a1a1a; border: 1px solid #388E3C; border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">beginner</span>
                            </div>
                        </button>
                        
                    </div>
                </section>
                
                <section class="strategy-section" style="margin-bottom: var(--space-8);">
                    <h2 class="section-title" style="font-family: var(--font-family-mono); font-size: var(--font-size-xl); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-4); text-align: center; color: var(--color-text-secondary);">statistical & factor models</h2>
                    <div class="strategy-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: var(--space-6);">
                        
                        <button class="strategy-card color-purple" onclick="selectStrategy('pairs-trading')" style="font-family: var(--font-family-sans); background: #9C27B0; border: 3px solid #7B1FA2; color: #ffffff; border-radius: var(--radius-lg); box-shadow: -3px 5px #7B1FA2; cursor: pointer; transition: all var(--transition-fast); padding: var(--space-4); text-align: left; position: relative; margin-bottom: 5px; margin-right: 5px; min-height: 140px; display: flex; flex-direction: column; width: 100%;">
                            <h3 class="strategy-title" style="font-family: var(--font-family-mono); font-size: var(--font-size-lg); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-2); line-height: 1.2;">Pairs Trading</h3>
                            <p class="strategy-description" style="font-size: var(--font-size-sm); color: #e0e0e0; line-height: 1.4; margin-bottom: var(--space-3); flex-grow: 1;">Long-short strategies exploiting divergences between correlated assets.</p>
                            <div class="strategy-tags" style="display: flex; flex-wrap: wrap; gap: var(--space-1); margin-top: auto;">
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.2); color: #ffffff; border: 1px solid rgba(255,255,255,0.3); border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">market neutral</span>
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.2); color: #ffffff; border: 1px solid rgba(255,255,255,0.3); border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">intermediate</span>
                            </div>
                        </button>
                        
                        <button class="strategy-card color-red" onclick="selectStrategy('volatility')" style="font-family: var(--font-family-sans); background: #F44336; border: 3px solid #D32F2F; color: #ffffff; border-radius: var(--radius-lg); box-shadow: -3px 5px #D32F2F; cursor: pointer; transition: all var(--transition-fast); padding: var(--space-4); text-align: left; position: relative; margin-bottom: 5px; margin-right: 5px; min-height: 140px; display: flex; flex-direction: column; width: 100%;">
                            <h3 class="strategy-title" style="font-family: var(--font-family-mono); font-size: var(--font-size-lg); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-2); line-height: 1.2;">Volatility Trading</h3>
                            <p class="strategy-description" style="font-size: var(--font-size-sm); color: #ffcdd2; line-height: 1.4; margin-bottom: var(--space-3); flex-grow: 1;">Advanced strategies focused on volatility surface dynamics and options.</p>
                            <div class="strategy-tags" style="display: flex; flex-wrap: wrap; gap: var(--space-1); margin-top: auto;">
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.2); color: #ffffff; border: 1px solid rgba(255,255,255,0.3); border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">options</span>
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.2); color: #ffffff; border: 1px solid rgba(255,255,255,0.3); border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">expert</span>
                            </div>
                        </button>
                        
                        <button class="strategy-card color-teal coming-soon" onclick="selectStrategy('statistical-arb')" style="font-family: var(--font-family-sans); background: #009688; border: 3px solid #00695C; color: #ffffff; border-radius: var(--radius-lg); box-shadow: -3px 5px #00695C; cursor: pointer; transition: all var(--transition-fast); padding: var(--space-4); text-align: left; position: relative; margin-bottom: 5px; margin-right: 5px; min-height: 140px; display: flex; flex-direction: column; width: 100%; opacity: 0.8;">
                            <h3 class="strategy-title" style="font-family: var(--font-family-mono); font-size: var(--font-size-lg); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-2); line-height: 1.2;">Statistical Arbitrage</h3>
                            <p class="strategy-description" style="font-size: var(--font-size-sm); color: #b2dfdb; line-height: 1.4; margin-bottom: var(--space-3); flex-grow: 1;">Multi-asset statistical models using factor decomposition and PCA.</p>
                            <div class="strategy-tags" style="display: flex; flex-wrap: wrap; gap: var(--space-1); margin-top: auto;">
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.2); color: #ffffff; border: 1px solid rgba(255,255,255,0.3); border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">factor models</span>
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.2); color: #ffffff; border: 1px solid rgba(255,255,255,0.3); border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">advanced</span>
                            </div>
                            <span style="position: absolute; top: var(--space-2); right: var(--space-2); font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-semibold); padding: var(--space-1) var(--space-2); background: #ffd700; color: #333; border: 2px solid #333; border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">Coming Soon</span>
                        </button>
                        
                    </div>
                </section>
                
                <section class="strategy-section" style="margin-bottom: var(--space-8);">
                    <h2 class="section-title" style="font-family: var(--font-family-mono); font-size: var(--font-size-xl); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-4); text-align: center; color: var(--color-text-secondary);">machine learning & portfolio</h2>
                    <div class="strategy-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: var(--space-6);">
                        
                        <button class="strategy-card color-indigo" onclick="selectStrategy('ml-predictive')" style="font-family: var(--font-family-sans); background: #3F51B5; border: 3px solid #303F9F; color: #ffffff; border-radius: var(--radius-lg); box-shadow: -3px 5px #303F9F; cursor: pointer; transition: all var(--transition-fast); padding: var(--space-4); text-align: left; position: relative; margin-bottom: 5px; margin-right: 5px; min-height: 140px; display: flex; flex-direction: column; width: 100%;">
                            <h3 class="strategy-title" style="font-family: var(--font-family-mono); font-size: var(--font-size-lg); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-2); line-height: 1.2;">ML Predictive Models</h3>
                            <p class="strategy-description" style="font-size: var(--font-size-sm); color: #c5cae9; line-height: 1.4; margin-bottom: var(--space-3); flex-grow: 1;">Ensemble models and deep learning for alpha generation.</p>
                            <div class="strategy-tags" style="display: flex; flex-wrap: wrap; gap: var(--space-1); margin-top: auto;">
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.2); color: #ffffff; border: 1px solid rgba(255,255,255,0.3); border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">deep learning</span>
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.2); color: #ffffff; border: 1px solid rgba(255,255,255,0.3); border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">expert</span>
                            </div>
                        </button>
                        
                        <button class="strategy-card color-pink coming-soon" onclick="selectStrategy('sentiment')" style="font-family: var(--font-family-sans); background: #E91E63; border: 3px solid #C2185B; color: #ffffff; border-radius: var(--radius-lg); box-shadow: -3px 5px #C2185B; cursor: pointer; transition: all var(--transition-fast); padding: var(--space-4); text-align: left; position: relative; margin-bottom: 5px; margin-right: 5px; min-height: 140px; display: flex; flex-direction: column; width: 100%; opacity: 0.8;">
                            <h3 class="strategy-title" style="font-family: var(--font-family-mono); font-size: var(--font-size-lg); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-2); line-height: 1.2;">Sentiment Analysis</h3>
                            <p class="strategy-description" style="font-size: var(--font-size-sm); color: #f8bbd9; line-height: 1.4; margin-bottom: var(--space-3); flex-grow: 1;">NLP-driven strategies using news and social media signals.</p>
                            <div class="strategy-tags" style="display: flex; flex-wrap: wrap; gap: var(--space-1); margin-top: auto;">
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.2); color: #ffffff; border: 1px solid rgba(255,255,255,0.3); border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">nlp</span>
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.2); color: #ffffff; border: 1px solid rgba(255,255,255,0.3); border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">advanced</span>
                            </div>
                            <span style="position: absolute; top: var(--space-2); right: var(--space-2); font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-semibold); padding: var(--space-1) var(--space-2); background: #ffd700; color: #333; border: 2px solid #333; border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">Coming Soon</span>
                        </button>
                        
                        <button class="strategy-card color-cyan" onclick="selectStrategy('custom')" style="font-family: var(--font-family-sans); background: #00BCD4; border: 3px solid #0097A7; color: #1a1a1a; border-radius: var(--radius-lg); box-shadow: -3px 5px #0097A7; cursor: pointer; transition: all var(--transition-fast); padding: var(--space-4); text-align: left; position: relative; margin-bottom: 5px; margin-right: 5px; min-height: 140px; display: flex; flex-direction: column; width: 100%;">
                            <h3 class="strategy-title" style="font-family: var(--font-family-mono); font-size: var(--font-size-lg); font-weight: var(--font-weight-semibold); margin-bottom: var(--space-2); line-height: 1.2;">Custom Strategy Builder</h3>
                            <p class="strategy-description" style="font-size: var(--font-size-sm); color: #2a2a2a; line-height: 1.4; margin-bottom: var(--space-3); flex-grow: 1;">Visual drag-and-drop interface for building custom strategies.</p>
                            <div class="strategy-tags" style="display: flex; flex-wrap: wrap; gap: var(--space-1); margin-top: auto;">
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.3); color: #1a1a1a; border: 1px solid #0097A7; border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">visual builder</span>
                                <span class="tag" style="font-family: var(--font-family-mono); font-size: var(--font-size-xs); font-weight: var(--font-weight-medium); padding: 2px var(--space-2); background: rgba(255,255,255,0.3); color: #1a1a1a; border: 1px solid #0097A7; border-radius: var(--radius-md); text-transform: uppercase; letter-spacing: 0.05em;">all levels</span>
                            </div>
                        </button>
                        
                    </div>
                </section>
            </div>'''

# Find the notebook container and replace it
import re
notebook_pattern = r'<!-- Notebook Container -->.*?</div>\s*(?=</div>\s*</body>|$)'
if re.search(notebook_pattern, content, re.DOTALL):
    content = re.sub(notebook_pattern, catalogue_html, content, flags=re.DOTALL)
    print("✅ Replaced notebook container with catalogue view")
else:
    print("❌ Could not find notebook container to replace")

# Write the updated content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.write(content)

print("✨ Research page now shows catalogue view by default")