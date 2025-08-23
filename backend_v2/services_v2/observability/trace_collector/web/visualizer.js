/**
 * AlphaPulse Message Flow Visualizer
 * Real-time distributed tracing visualization using D3.js
 */

class MessageFlowVisualizer {
    constructor() {
        this.apiBaseUrl = 'http://localhost:8080'; // TraceCollector API
        this.isPlaying = true;
        this.traces = new Map();
        this.activeFlows = new Set();
        this.updateInterval = null;
        
        // Service definitions
        this.services = [
            { id: 'polygon', name: 'Polygon Collector', x: 100, y: 200, color: '#3b82f6' },
            { id: 'relay', name: 'Market Data Relay', x: 400, y: 200, color: '#8b5cf6' },
            { id: 'strategy', name: 'Arbitrage Strategy', x: 700, y: 200, color: '#10b981' },
            { id: 'dashboard', name: 'Dashboard', x: 700, y: 350, color: '#f59e0b' }
        ];
        
        this.connections = [
            { source: 'polygon', target: 'relay' },
            { source: 'relay', target: 'strategy' },
            { source: 'relay', target: 'dashboard' }
        ];
        
        this.stats = {
            totalTraces: 0,
            avgLatency: 0,
            messagesPerSec: 0,
            errorRate: 0,
            serviceMetrics: {}
        };
        
        this.initializeVisualization();
        this.setupEventListeners();
        this.startDataPolling();
    }
    
    initializeVisualization() {
        const svg = d3.select('#flow-svg');
        const rect = svg.node().getBoundingClientRect();
        
        svg.attr('width', rect.width).attr('height', rect.height);
        
        // Create arrow markers for flow direction
        svg.append('defs').append('marker')
            .attr('id', 'arrowhead')
            .attr('viewBox', '0 -5 10 10')
            .attr('refX', 15)
            .attr('refY', 0)
            .attr('markerWidth', 6)
            .attr('markerHeight', 6)
            .attr('orient', 'auto')
            .append('path')
            .attr('d', 'M0,-5L10,0L0,5')
            .attr('fill', '#4ade80');
        
        // Create connections (links)
        const linkGroup = svg.append('g').attr('class', 'links');
        linkGroup.selectAll('.link')
            .data(this.connections)
            .enter()
            .append('path')
            .attr('class', 'link')
            .attr('id', d => `link-${d.source}-${d.target}`)
            .attr('marker-end', 'url(#arrowhead)')
            .attr('d', d => this.createLinkPath(d));
        
        // Create service nodes
        const nodeGroup = svg.append('g').attr('class', 'nodes');
        const nodes = nodeGroup.selectAll('.node-group')
            .data(this.services)
            .enter()
            .append('g')
            .attr('class', 'node-group')
            .attr('transform', d => `translate(${d.x}, ${d.y})`);
        
        // Service circles
        nodes.append('circle')
            .attr('class', 'node')
            .attr('id', d => `node-${d.id}`)
            .attr('r', 40)
            .style('fill', d => d.color)
            .on('click', (event, d) => this.showServiceDetails(d))
            .on('mouseover', (event, d) => this.highlightService(d.id))
            .on('mouseout', () => this.clearHighlight());
        
        // Service labels
        nodes.append('text')
            .attr('class', 'node-label')
            .attr('dy', 5)
            .text(d => d.name.split(' ')[0]); // First word only for space
        
        // Add pulse animation container
        svg.append('g').attr('class', 'pulses');
        
        this.svg = svg;
        document.getElementById('loading').style.display = 'none';
    }
    
    createLinkPath(d) {
        const source = this.services.find(s => s.id === d.source);
        const target = this.services.find(s => s.id === d.target);
        
        const dx = target.x - source.x;
        const dy = target.y - source.y;
        const dr = Math.sqrt(dx * dx + dy * dy) * 0.3;
        
        return `M${source.x},${source.y}A${dr},${dr} 0 0,1 ${target.x},${target.y}`;
    }
    
    async startDataPolling() {
        if (this.updateInterval) clearInterval(this.updateInterval);
        
        this.updateInterval = setInterval(async () => {
            if (this.isPlaying) {
                await this.fetchTraceData();
                await this.fetchHealthData();
                this.updateVisualization();
            }
        }, 1000); // Update every second
        
        // Initial fetch
        await this.fetchTraceData();
        await this.fetchHealthData();
        this.updateVisualization();
    }
    
    async fetchTraceData() {
        try {
            const response = await axios.get(`${this.apiBaseUrl}/api/traces?limit=20`);
            
            if (response.data && response.data.data) {
                response.data.data.forEach(trace => {
                    this.traces.set(trace.trace_id, trace);
                    
                    // Add to active flows if it's recent
                    const age = Date.now() - (trace.start_time_ns / 1000000);
                    if (age < 5000 && !trace.has_errors) { // 5 seconds
                        this.activeFlows.add(trace.trace_id);
                        
                        // Remove from active flows after animation
                        setTimeout(() => {
                            this.activeFlows.delete(trace.trace_id);
                        }, 3000);
                    }
                });
                
                this.updateStats(response.data.data);
            }
        } catch (error) {
            console.warn('Failed to fetch trace data:', error.message);
            this.updateConnectionStatus('error');
        }
    }
    
    async fetchHealthData() {
        try {
            const response = await axios.get(`${this.apiBaseUrl}/api/health`);
            
            if (response.data && response.data.data) {
                this.updateServiceHealth(response.data.data);
            }
        } catch (error) {
            console.warn('Failed to fetch health data:', error.message);
        }
    }
    
    updateStats(traces) {
        this.stats.totalTraces = traces.length;
        
        // Calculate average latency
        const completedTraces = traces.filter(t => t.status === 'completed');
        if (completedTraces.length > 0) {
            const totalLatency = completedTraces.reduce((sum, t) => sum + t.duration_ms, 0);
            this.stats.avgLatency = Math.round(totalLatency / completedTraces.length);
        }
        
        // Calculate messages per second (approximate)
        const recentTraces = traces.filter(t => {
            const age = Date.now() - (t.start_time_ns / 1000000);
            return age < 60000; // Last minute
        });
        this.stats.messagesPerSec = Math.round(recentTraces.length / 60);
        
        // Calculate error rate
        const errorTraces = traces.filter(t => t.has_errors);
        this.stats.errorRate = traces.length > 0 
            ? Math.round((errorTraces.length / traces.length) * 100) 
            : 0;
        
        // Update DOM
        document.getElementById('total-traces').textContent = this.stats.totalTraces;
        document.getElementById('avg-latency').textContent = `${this.stats.avgLatency}ms`;
        document.getElementById('messages-per-sec').textContent = this.stats.messagesPerSec;
        document.getElementById('error-rate').textContent = `${this.stats.errorRate}%`;
    }
    
    updateServiceHealth(health) {
        // Update service status indicators
        const services = ['polygon', 'relay', 'strategy'];
        
        services.forEach(service => {
            const statusElement = document.getElementById(`${service}-status`);
            const statusTextElement = document.getElementById(`${service}-status-text`);
            
            if (health.status === 'Healthy') {
                statusElement.className = 'status-indicator';
                statusTextElement.textContent = 'Healthy';
            } else if (health.status === 'Degraded') {
                statusElement.className = 'status-indicator warning';
                statusTextElement.textContent = 'Degraded';
            } else {
                statusElement.className = 'status-indicator error';
                statusTextElement.textContent = 'Unhealthy';
            }
        });
        
        // Update service metrics (mock data for now)
        document.getElementById('polygon-rate').textContent = Math.round(Math.random() * 50);
        document.getElementById('polygon-total').textContent = Math.round(Math.random() * 10000);
        document.getElementById('polygon-latency').textContent = `${Math.round(Math.random() * 20)}ms`;
        
        document.getElementById('relay-consumers').textContent = Math.round(Math.random() * 5) + 1;
        document.getElementById('relay-rate').textContent = Math.round(Math.random() * 100);
        document.getElementById('relay-queue').textContent = Math.round(Math.random() * 10);
        
        document.getElementById('strategy-opportunities').textContent = Math.round(Math.random() * 20);
        document.getElementById('strategy-success').textContent = `${Math.round(Math.random() * 30 + 70)}%`;
        document.getElementById('strategy-profit').textContent = `$${(Math.random() * 500).toFixed(2)}`;
    }
    
    updateVisualization() {
        // Animate active flows
        this.activeFlows.forEach(traceId => {
            this.animateFlow(traceId);
        });
        
        // Update service node states
        this.services.forEach(service => {
            const node = this.svg.select(`#node-${service.id}`);
            const hasActiveFlow = Array.from(this.activeFlows).some(traceId => {
                const trace = this.traces.get(traceId);
                return trace && this.traceInvolvesService(trace, service.id);
            });
            
            node.classed('active', hasActiveFlow);
        });
    }
    
    animateFlow(traceId) {
        const trace = this.traces.get(traceId);
        if (!trace) return;
        
        // Determine the flow path based on trace origin and destination
        let path = [];
        if (trace.origin === 'PolygonCollector') {
            path = ['polygon', 'relay'];
            if (trace.execution_triggered) {
                path.push('strategy');
            } else {
                path.push('dashboard');
            }
        }
        
        // Animate each connection in the path
        for (let i = 0; i < path.length - 1; i++) {
            const source = path[i];
            const target = path[i + 1];
            const linkId = `#link-${source}-${target}`;
            
            this.svg.select(linkId)
                .classed('active', true)
                .attr('stroke-dasharray', '10,10');
            
            // Remove animation after 2 seconds
            setTimeout(() => {
                this.svg.select(linkId)
                    .classed('active', false)
                    .attr('stroke-dasharray', null);
            }, 2000);
        }
        
        // Add pulse effect to involved nodes
        path.forEach((serviceId, index) => {
            setTimeout(() => {
                this.createPulseEffect(serviceId);
            }, index * 500);
        });
    }
    
    createPulseEffect(serviceId) {
        const service = this.services.find(s => s.id === serviceId);
        if (!service) return;
        
        const pulseGroup = this.svg.select('.pulses');
        
        const pulse = pulseGroup.append('circle')
            .attr('cx', service.x)
            .attr('cy', service.y)
            .attr('r', 40)
            .attr('fill', 'none')
            .attr('stroke', '#4ade80')
            .attr('stroke-width', 2)
            .attr('opacity', 0.8);
        
        pulse.transition()
            .duration(1000)
            .attr('r', 80)
            .attr('opacity', 0)
            .remove();
    }
    
    traceInvolvesService(trace, serviceId) {
        const serviceMap = {
            'polygon': 'PolygonCollector',
            'relay': 'MarketDataRelay',
            'strategy': 'ArbitrageStrategy',
            'dashboard': 'Dashboard'
        };
        
        return trace.origin === serviceMap[serviceId] ||
               (trace.destination && trace.destination === serviceMap[serviceId]) ||
               (trace.services_traversed && trace.services_traversed.includes(serviceMap[serviceId]));
    }
    
    highlightService(serviceId) {
        // Highlight the service and its connections
        this.svg.select(`#node-${serviceId}`)
            .style('stroke-width', '4px');
        
        // Highlight connected links
        this.connections.forEach(conn => {
            if (conn.source === serviceId || conn.target === serviceId) {
                this.svg.select(`#link-${conn.source}-${conn.target}`)
                    .style('stroke-width', '4px')
                    .style('opacity', '1');
            }
        });
    }
    
    clearHighlight() {
        this.svg.selectAll('.node')
            .style('stroke-width', '2px');
        
        this.svg.selectAll('.link')
            .style('stroke-width', '2px')
            .style('opacity', '0.6');
    }
    
    showServiceDetails(service) {
        // Show service details in a popup or panel
        console.log('Service details:', service);
        
        // Create a popup showing recent traces for this service
        const recentTraces = Array.from(this.traces.values())
            .filter(trace => this.traceInvolvesService(trace, service.id))
            .slice(0, 5);
        
        let details = `<div class="trace-popup" style="left: ${service.x + 60}px; top: ${service.y - 30}px;">
            <div class="trace-id">${service.name}</div>
            <div>Recent Traces: ${recentTraces.length}</div>
        `;
        
        recentTraces.forEach(trace => {
            details += `<div style="margin-top: 6px; font-size: 11px;">
                ${trace.trace_id.substring(0, 8)}... - ${trace.duration_ms}ms
                ${trace.execution_triggered ? ' 🎯' : ''}
                ${trace.has_errors ? ' ❌' : ' ✅'}
            </div>`;
        });
        
        details += '</div>';
        
        // Remove existing popup
        document.querySelectorAll('.trace-popup').forEach(p => p.remove());
        
        // Add new popup
        document.querySelector('.flow-canvas').insertAdjacentHTML('beforeend', details);
        
        // Remove popup after 3 seconds
        setTimeout(() => {
            document.querySelectorAll('.trace-popup').forEach(p => p.remove());
        }, 3000);
    }
    
    setupEventListeners() {
        // Play/Pause button
        document.getElementById('play-pause').addEventListener('click', () => {
            this.isPlaying = !this.isPlaying;
            const btn = document.getElementById('play-pause');
            btn.textContent = this.isPlaying ? '⏸️ Pause' : '▶️ Play';
            btn.classList.toggle('active', !this.isPlaying);
        });
        
        // Clear traces button
        document.getElementById('clear-traces').addEventListener('click', () => {
            this.traces.clear();
            this.activeFlows.clear();
            this.clearVisualization();
        });
        
        // Export data button
        document.getElementById('export-data').addEventListener('click', () => {
            this.exportTraceData();
        });
        
        // Handle window resize
        window.addEventListener('resize', () => {
            this.resizeVisualization();
        });
    }
    
    clearVisualization() {
        this.svg.selectAll('.link').classed('active', false);
        this.svg.selectAll('.node').classed('active', false);
        this.svg.select('.pulses').selectAll('*').remove();
        
        // Reset stats
        document.getElementById('total-traces').textContent = '0';
        document.getElementById('avg-latency').textContent = '0ms';
        document.getElementById('messages-per-sec').textContent = '0';
        document.getElementById('error-rate').textContent = '0%';
    }
    
    exportTraceData() {
        const data = {
            timestamp: new Date().toISOString(),
            traces: Array.from(this.traces.values()),
            stats: this.stats
        };
        
        const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        
        const a = document.createElement('a');
        a.href = url;
        a.download = `alphapulse-traces-${Date.now()}.json`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }
    
    resizeVisualization() {
        const svg = d3.select('#flow-svg');
        const rect = svg.node().getBoundingClientRect();
        svg.attr('width', rect.width).attr('height', rect.height);
    }
    
    updateConnectionStatus(status) {
        // Update connection status indicators
        const statusMap = {
            'connected': { class: 'status-indicator', text: 'Connected' },
            'error': { class: 'status-indicator error', text: 'Connection Error' },
            'connecting': { class: 'status-indicator warning', text: 'Connecting...' }
        };
        
        const statusInfo = statusMap[status] || statusMap['error'];
        
        ['polygon', 'relay', 'strategy'].forEach(service => {
            const statusEl = document.getElementById(`${service}-status`);
            const textEl = document.getElementById(`${service}-status-text`);
            
            if (statusEl && textEl) {
                statusEl.className = statusInfo.class;
                textEl.textContent = statusInfo.text;
            }
        });
    }
}

// Initialize the visualizer when the DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    console.log('🚀 Initializing AlphaPulse Message Flow Visualizer...');
    const visualizer = new MessageFlowVisualizer();
    
    // Make visualizer globally available for debugging
    window.visualizer = visualizer;
});