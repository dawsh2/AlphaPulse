import React, { useState } from 'react';
import styles from './StrategyExporter.module.css';

interface ExportConfig {
  strategyId: string;
  strategyName: string;
  type: 'single' | 'ensemble';
  components?: Array<{
    strategyId: string;
    weight: number;
    regimeConditions?: string;
  }>;
  parameters: Record<string, any>;
  entryConditions: string[];
  exitConditions: string[];
  riskManagement: {
    stopLoss?: string;
    takeProfit?: string;
    positionSizing?: string;
  };
  backtestResults?: {
    sharpe: number;
    maxDrawdown: number;
    winRate: number;
    totalReturn: number;
  };
  metadata: {
    createdAt: string;
    lastModified: string;
    author?: string;
    description?: string;
    tags?: string[];
  };
}

interface StrategyExporterProps {
  config: ExportConfig;
  onClose: () => void;
  onSave?: (config: ExportConfig) => void;
}

export const StrategyExporter: React.FC<StrategyExporterProps> = ({ config, onClose, onSave }) => {
  const [exportFormat, setExportFormat] = useState<'json' | 'python' | 'yaml' | 'clipboard'>('json');
  const [saveLocation, setSaveLocation] = useState<'local' | 'cloud' | 'github'>('local');
  const [strategyName, setStrategyName] = useState(config.strategyName);
  const [description, setDescription] = useState(config.metadata.description || '');
  const [tags, setTags] = useState(config.metadata.tags?.join(', ') || '');
  const [isPublic, setIsPublic] = useState(false);
  const [saveAsTemplate, setSaveAsTemplate] = useState(false);
  
  // Export formats
  const generateJSON = () => {
    return JSON.stringify(config, null, 2);
  };
  
  const generatePython = () => {
    const pythonCode = `
# AlphaPulse Strategy: ${config.strategyName}
# Generated: ${new Date().toISOString()}
# Performance: Sharpe ${config.backtestResults?.sharpe.toFixed(2)}, Max DD ${config.backtestResults?.maxDrawdown.toFixed(1)}%

from alphapulse import Strategy, Condition, RiskManager

class ${config.strategyName.replace(/\s+/g, '')}(Strategy):
    """
    ${config.metadata.description || 'Auto-generated strategy from AlphaPulse'}
    
    Backtest Results:
    - Sharpe Ratio: ${config.backtestResults?.sharpe.toFixed(2)}
    - Max Drawdown: ${config.backtestResults?.maxDrawdown.toFixed(1)}%
    - Win Rate: ${config.backtestResults?.winRate.toFixed(1)}%
    - Total Return: ${config.backtestResults?.totalReturn.toFixed(1)}%
    """
    
    def __init__(self):
        super().__init__(name="${config.strategyName}")
        
        # Parameters
        self.parameters = ${JSON.stringify(config.parameters, null, 8).replace(/"/g, "'")}
        
        # Entry Conditions
        self.entry_conditions = [
${config.entryConditions.map(c => `            Condition("${c}")`).join(',\n')}
        ]
        
        # Exit Conditions
        self.exit_conditions = [
${config.exitConditions.map(c => `            Condition("${c}")`).join(',\n')}
        ]
        
        # Risk Management
        self.risk_manager = RiskManager(
            stop_loss="${config.riskManagement.stopLoss || 'None'}",
            take_profit="${config.riskManagement.takeProfit || 'None'}",
            position_sizing="${config.riskManagement.positionSizing || '0.1'}"
        )
    
    def generate_signals(self, data):
        """Generate trading signals based on defined conditions"""
        signals = []
        
        for condition in self.entry_conditions:
            if condition.evaluate(data):
                signals.append(1)  # Long signal
        
        for condition in self.exit_conditions:
            if condition.evaluate(data):
                signals.append(0)  # Exit signal
        
        return signals

# Ensemble Configuration (if applicable)
${config.type === 'ensemble' ? `
ensemble_components = ${JSON.stringify(config.components, null, 4).replace(/"/g, "'")}

def create_ensemble():
    """Create an ensemble of strategies with regime-based allocation"""
    from alphapulse import EnsembleStrategy
    
    ensemble = EnsembleStrategy(
        name="${config.strategyName}_Ensemble",
        components=ensemble_components
    )
    return ensemble
` : '# This is a single strategy, not an ensemble'}
`;
    return pythonCode;
  };
  
  const generateYAML = () => {
    const yamlContent = `
# AlphaPulse Strategy Configuration
# Generated: ${new Date().toISOString()}

strategy:
  name: ${config.strategyName}
  type: ${config.type}
  author: ${config.metadata.author || 'AlphaPulse User'}
  description: ${config.metadata.description || 'Strategy discovered via AlphaPulse Explorer'}
  tags: ${config.metadata.tags?.join(', ') || ''}

parameters:
${Object.entries(config.parameters).map(([key, value]) => `  ${key}: ${value}`).join('\n')}

conditions:
  entry:
${config.entryConditions.map(c => `    - "${c}"`).join('\n')}
  exit:
${config.exitConditions.map(c => `    - "${c}"`).join('\n')}

risk_management:
  stop_loss: ${config.riskManagement.stopLoss || 'null'}
  take_profit: ${config.riskManagement.takeProfit || 'null'}
  position_sizing: ${config.riskManagement.positionSizing || '0.1'}

backtest_results:
  sharpe_ratio: ${config.backtestResults?.sharpe.toFixed(3)}
  max_drawdown: ${config.backtestResults?.maxDrawdown.toFixed(2)}
  win_rate: ${config.backtestResults?.winRate.toFixed(2)}
  total_return: ${config.backtestResults?.totalReturn.toFixed(2)}

${config.type === 'ensemble' ? `
ensemble:
  components:
${config.components?.map(c => `    - strategy: ${c.strategyId}
      weight: ${c.weight}
      regime: ${c.regimeConditions || 'all'}`).join('\n')}
` : ''}

metadata:
  created_at: ${config.metadata.createdAt}
  last_modified: ${config.metadata.lastModified}
  version: 1.0.0
`;
    return yamlContent;
  };
  
  const handleExport = async () => {
    let content = '';
    let filename = `${strategyName.replace(/\s+/g, '_').toLowerCase()}`;
    
    switch (exportFormat) {
      case 'json':
        content = generateJSON();
        filename += '.json';
        break;
      case 'python':
        content = generatePython();
        filename += '.py';
        break;
      case 'yaml':
        content = generateYAML();
        filename += '.yaml';
        break;
      case 'clipboard':
        content = generateJSON();
        await navigator.clipboard.writeText(content);
        alert('Strategy copied to clipboard!');
        return;
    }
    
    // Download file
    const blob = new Blob([content], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };
  
  const handleSave = async () => {
    const updatedConfig = {
      ...config,
      strategyName,
      metadata: {
        ...config.metadata,
        description,
        tags: tags.split(',').map(t => t.trim()).filter(Boolean),
        lastModified: new Date().toISOString()
      }
    };
    
    switch (saveLocation) {
      case 'local':
        // Save to localStorage
        const savedStrategies = JSON.parse(localStorage.getItem('alphapulse_strategies') || '[]');
        savedStrategies.push(updatedConfig);
        localStorage.setItem('alphapulse_strategies', JSON.stringify(savedStrategies));
        
        if (saveAsTemplate) {
          const templates = JSON.parse(localStorage.getItem('alphapulse_templates') || '[]');
          templates.push({
            ...updatedConfig,
            isTemplate: true,
            templateId: `template_${Date.now()}`
          });
          localStorage.setItem('alphapulse_templates', JSON.stringify(templates));
        }
        break;
        
      case 'cloud':
        // Save to backend
        try {
          const response = await fetch('/api/strategies', {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
              'Authorization': `Bearer ${localStorage.getItem('token')}`
            },
            body: JSON.stringify({
              ...updatedConfig,
              isPublic,
              saveAsTemplate
            })
          });
          
          if (!response.ok) throw new Error('Failed to save strategy');
          
          const result = await response.json();
          console.log('Strategy saved:', result);
        } catch (error) {
          console.error('Error saving strategy:', error);
          alert('Failed to save strategy to cloud. Saved locally instead.');
          // Fallback to local storage
          handleSave();
        }
        break;
        
      case 'github':
        // Create GitHub Gist
        try {
          const gistData = {
            description: `AlphaPulse Strategy: ${strategyName}`,
            public: isPublic,
            files: {
              [`${strategyName.replace(/\s+/g, '_')}.json`]: {
                content: generateJSON()
              },
              [`${strategyName.replace(/\s+/g, '_')}.py`]: {
                content: generatePython()
              },
              [`${strategyName.replace(/\s+/g, '_')}.yaml`]: {
                content: generateYAML()
              }
            }
          };
          
          // You would need GitHub token for this
          const githubToken = localStorage.getItem('github_token');
          if (!githubToken) {
            alert('Please connect your GitHub account first');
            return;
          }
          
          const response = await fetch('https://api.github.com/gists', {
            method: 'POST',
            headers: {
              'Authorization': `token ${githubToken}`,
              'Content-Type': 'application/json'
            },
            body: JSON.stringify(gistData)
          });
          
          if (!response.ok) throw new Error('Failed to create Gist');
          
          const gist = await response.json();
          window.open(gist.html_url, '_blank');
        } catch (error) {
          console.error('Error creating Gist:', error);
          alert('Failed to save to GitHub');
        }
        break;
    }
    
    if (onSave) {
      onSave(updatedConfig);
    }
    
    alert('Strategy saved successfully!');
  };
  
  return (
    <div className={styles.exporterOverlay}>
      <div className={styles.exporterModal}>
        <div className={styles.header}>
          <h2>💾 Save & Export Strategy</h2>
          <button className={styles.closeBtn} onClick={onClose}>×</button>
        </div>
        
        <div className={styles.content}>
          {/* Strategy Info */}
          <div className={styles.section}>
            <h3>Strategy Information</h3>
            <div className={styles.formGroup}>
              <label>Strategy Name</label>
              <input
                type="text"
                value={strategyName}
                onChange={(e) => setStrategyName(e.target.value)}
                className={styles.input}
              />
            </div>
            <div className={styles.formGroup}>
              <label>Description</label>
              <textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                className={styles.textarea}
                placeholder="Describe your strategy..."
              />
            </div>
            <div className={styles.formGroup}>
              <label>Tags (comma-separated)</label>
              <input
                type="text"
                value={tags}
                onChange={(e) => setTags(e.target.value)}
                className={styles.input}
                placeholder="momentum, mean-reversion, high-frequency"
              />
            </div>
          </div>
          
          {/* Performance Summary */}
          {config.backtestResults && (
            <div className={styles.section}>
              <h3>Performance Summary</h3>
              <div className={styles.metricsGrid}>
                <div className={styles.metric}>
                  <span className={styles.metricLabel}>Sharpe Ratio</span>
                  <span className={styles.metricValue}>{config.backtestResults.sharpe.toFixed(2)}</span>
                </div>
                <div className={styles.metric}>
                  <span className={styles.metricLabel}>Max Drawdown</span>
                  <span className={styles.metricValue}>{config.backtestResults.maxDrawdown.toFixed(1)}%</span>
                </div>
                <div className={styles.metric}>
                  <span className={styles.metricLabel}>Win Rate</span>
                  <span className={styles.metricValue}>{config.backtestResults.winRate.toFixed(1)}%</span>
                </div>
                <div className={styles.metric}>
                  <span className={styles.metricLabel}>Total Return</span>
                  <span className={styles.metricValue}>{config.backtestResults.totalReturn.toFixed(1)}%</span>
                </div>
              </div>
            </div>
          )}
          
          {/* Export Options */}
          <div className={styles.section}>
            <h3>Export Format</h3>
            <div className={styles.formatOptions}>
              <button
                className={`${styles.formatBtn} ${exportFormat === 'json' ? styles.active : ''}`}
                onClick={() => setExportFormat('json')}
              >
                <span className={styles.formatIcon}>{ }</span>
                JSON
              </button>
              <button
                className={`${styles.formatBtn} ${exportFormat === 'python' ? styles.active : ''}`}
                onClick={() => setExportFormat('python')}
              >
                <span className={styles.formatIcon}>🐍</span>
                Python
              </button>
              <button
                className={`${styles.formatBtn} ${exportFormat === 'yaml' ? styles.active : ''}`}
                onClick={() => setExportFormat('yaml')}
              >
                <span className={styles.formatIcon}>📄</span>
                YAML
              </button>
              <button
                className={`${styles.formatBtn} ${exportFormat === 'clipboard' ? styles.active : ''}`}
                onClick={() => setExportFormat('clipboard')}
              >
                <span className={styles.formatIcon}>📋</span>
                Clipboard
              </button>
            </div>
          </div>
          
          {/* Save Options */}
          <div className={styles.section}>
            <h3>Save Location</h3>
            <div className={styles.saveOptions}>
              <button
                className={`${styles.saveBtn} ${saveLocation === 'local' ? styles.active : ''}`}
                onClick={() => setSaveLocation('local')}
              >
                <span className={styles.saveIcon}>💻</span>
                <div>
                  <div className={styles.saveBtnTitle}>Local Storage</div>
                  <div className={styles.saveBtnDesc}>Save to browser</div>
                </div>
              </button>
              <button
                className={`${styles.saveBtn} ${saveLocation === 'cloud' ? styles.active : ''}`}
                onClick={() => setSaveLocation('cloud')}
              >
                <span className={styles.saveIcon}>☁️</span>
                <div>
                  <div className={styles.saveBtnTitle}>Cloud</div>
                  <div className={styles.saveBtnDesc}>Sync across devices</div>
                </div>
              </button>
              <button
                className={`${styles.saveBtn} ${saveLocation === 'github' ? styles.active : ''}`}
                onClick={() => setSaveLocation('github')}
              >
                <span className={styles.saveIcon}>🐙</span>
                <div>
                  <div className={styles.saveBtnTitle}>GitHub</div>
                  <div className={styles.saveBtnDesc}>Create Gist</div>
                </div>
              </button>
            </div>
            
            <div className={styles.saveOptionsExtra}>
              <label className={styles.checkbox}>
                <input
                  type="checkbox"
                  checked={saveAsTemplate}
                  onChange={(e) => setSaveAsTemplate(e.target.checked)}
                />
                Save as reusable template
              </label>
              {saveLocation !== 'local' && (
                <label className={styles.checkbox}>
                  <input
                    type="checkbox"
                    checked={isPublic}
                    onChange={(e) => setIsPublic(e.target.checked)}
                  />
                  Make publicly visible
                </label>
              )}
            </div>
          </div>
        </div>
        
        <div className={styles.footer}>
          <button className={styles.cancelBtn} onClick={onClose}>
            Cancel
          </button>
          <div className={styles.footerActions}>
            <button className={styles.exportBtn} onClick={handleExport}>
              ⬇️ Export File
            </button>
            <button className={styles.primaryBtn} onClick={handleSave}>
              💾 Save Strategy
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default StrategyExporter;