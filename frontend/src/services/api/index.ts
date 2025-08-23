/**
 * Centralized API Service Layer
 * 
 * This module provides a backend-agnostic interface for all API operations.
 * To switch backends, only the implementations in this file need to change.
 */

import { 
  type ApiConfig, 
  type ApiResponse, 
  ApiError,
  type AnalysisManifest,
  type BacktestResult,
  type BacktestParams,
  type Strategy,
  type Signal,
  type SignalRequest,
  type Position,
  type Order,
  type OrderRequest,
  type OrderParams,
  type MarketBar,
  type ButtonTemplate,
  type NotebookTemplate,
  type Dataset,
  type DatasetMetadata,
  type Event,
  type EventParams,
  type AccountInfo,
  type CompileResult
} from './types';

// Configuration - single source of truth for backend
const API_CONFIG: ApiConfig = {
  baseUrl: import.meta.env.VITE_API_URL || 'http://localhost:8000/api',
  wsUrl: import.meta.env.VITE_WS_URL || 'ws://localhost:8080/ws',
  timeout: 30000,
  retryAttempts: 3,
  retryDelay: 1000,
};

// Token management
let authToken: string | null = localStorage.getItem('auth_token');

/**
 * Base HTTP client with error handling and retries
 */
class HttpClient {
  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<ApiResponse<T>> {
    const url = `${API_CONFIG.baseUrl}${endpoint}`;
    
    const config: RequestInit = {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...(authToken && { Authorization: `Bearer ${authToken}` }),
        ...options.headers,
      },
    };

    try {
      const response = await fetch(url, config);
      
      if (!response.ok) {
        throw new ApiError(response.status, response.statusText);
      }
      
      const data = await response.json();
      return { success: true, data };
    } catch (error) {
      if (error instanceof ApiError) {
        throw error;
      }
      throw new ApiError(500, error instanceof Error ? error.message : 'Unknown error');
    }
  }

  async get<T>(endpoint: string): Promise<T> {
    const response = await this.request<T>(endpoint, { method: 'GET' });
    return response.data;
  }

  async post<T>(endpoint: string, body?: any): Promise<T> {
    const response = await this.request<T>(endpoint, {
      method: 'POST',
      body: JSON.stringify(body),
    });
    return response.data;
  }

  async put<T>(endpoint: string, body?: any): Promise<T> {
    const response = await this.request<T>(endpoint, {
      method: 'PUT',
      body: JSON.stringify(body),
    });
    return response.data;
  }

  async delete<T>(endpoint: string): Promise<T> {
    const response = await this.request<T>(endpoint, { method: 'DELETE' });
    return response.data;
  }
}

const http = new HttpClient();

/**
 * Authentication API
 */
export const auth = {
  async login(email: string, password: string) {
    const response = await http.post<{ token: string; user: any }>('/auth/login', {
      email,
      password,
    });
    authToken = response.token;
    localStorage.setItem('auth_token', authToken);
    return response;
  },

  async demoLogin() {
    const response = await http.post<{ token: string; user: any }>('/auth/demo-login');
    authToken = response.token;
    localStorage.setItem('auth_token', authToken);
    return response;
  },

  logout() {
    authToken = null;
    localStorage.removeItem('auth_token');
  },

  getToken() {
    return authToken;
  },
};

/**
 * Analysis & Backtesting API
 */
export const analysis = {
  async runAnalysis(manifest: AnalysisManifest): Promise<BacktestResult> {
    return http.post('/analysis/run', { manifest });
  },

  async checkCache(hash: string): Promise<{ exists: boolean; ttl: number }> {
    return http.get(`/analysis/cache/${hash}`);
  },

  async computeSignals(params: SignalRequest): Promise<Signal[]> {
    return http.post('/analysis/signals', params);
  },
};

/**
 * Strategy Management API
 */
export const strategies = {
  async list(): Promise<Strategy[]> {
    return http.get('/strategies');
  },

  async create(strategy: Partial<Strategy>): Promise<Strategy> {
    return http.post('/strategies', strategy);
  },

  async update(id: string, strategy: Partial<Strategy>): Promise<Strategy> {
    return http.put(`/strategies/${id}`, strategy);
  },

  async delete(id: string): Promise<void> {
    return http.delete(`/strategies/${id}`);
  },

  async backtest(id: string, params: BacktestParams): Promise<BacktestResult> {
    return http.post(`/strategies/${id}/backtest`, params);
  },

  async compile(id: string, code: string): Promise<CompileResult> {
    return http.post(`/strategies/${id}/compile`, { code });
  },
};

/**
 * Market Data API
 */
export const marketData = {
  async getBars(
    symbol: string,
    timeframe: string,
    limit: number = 100
  ): Promise<MarketBar[]> {
    return http.get(`/market-data/${symbol}?timeframe=${timeframe}&limit=${limit}`);
  },

  async saveData(data: any): Promise<void> {
    return http.post('/market-data/save', data);
  },

  // WebSocket connection for live data
  connectLive(
    symbols: string[],
    onMessage: (data: any) => void
  ): WebSocket {
    const ws = new WebSocket(`${API_CONFIG.wsUrl}/market-data`);
    
    ws.onopen = () => {
      ws.send(JSON.stringify({ action: 'subscribe', symbols }));
    };
    
    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      onMessage(data);
    };
    
    return ws;
  },
};

/**
 * Trading Operations API
 */
export const trading = {
  async getPositions(): Promise<Position[]> {
    return http.get('/positions');
  },

  async getOrders(params?: OrderParams): Promise<Order[]> {
    const query = new URLSearchParams(params as any).toString();
    return http.get(`/orders${query ? `?${query}` : ''}`);
  },

  async submitOrder(order: OrderRequest): Promise<Order> {
    return http.post('/orders', order);
  },

  async cancelOrder(id: string): Promise<void> {
    return http.delete(`/orders/${id}`);
  },

  async getAccount(): Promise<AccountInfo> {
    return http.get('/account');
  },
};

/**
 * Templates & Button-UI API
 */
export const templates = {
  async getButtonTemplates(): Promise<ButtonTemplate[]> {
    return http.get('/templates/button-ui');
  },

  async saveButtonTemplate(template: ButtonTemplate): Promise<ButtonTemplate> {
    return http.post('/templates/button-ui', template);
  },

  async getNotebookTemplates(): Promise<NotebookTemplate[]> {
    return http.get('/templates/notebook');
  },

  async saveNotebookTemplate(template: NotebookTemplate): Promise<NotebookTemplate> {
    return http.post('/templates/notebook', template);
  },
};

/**
 * Data Management API
 */
export const dataManagement = {
  async listDatasets(): Promise<Dataset[]> {
    return http.get('/data/datasets');
  },

  async uploadDataset(file: File, metadata: DatasetMetadata): Promise<Dataset> {
    const formData = new FormData();
    formData.append('file', file);
    formData.append('metadata', JSON.stringify(metadata));
    
    const response = await fetch(`${API_CONFIG.baseUrl}/data/upload`, {
      method: 'POST',
      headers: {
        ...(authToken && { Authorization: `Bearer ${authToken}` }),
      },
      body: formData,
    });
    
    return response.json();
  },

  async queryDataset(id: string, query: string): Promise<any[]> {
    return http.post(`/data/datasets/${id}/query`, { query });
  },
};

/**
 * Event Stream API
 */
export const events = {
  connect(
    eventTypes: string[],
    onEvent: (event: any) => void
  ): WebSocket {
    const ws = new WebSocket(`${API_CONFIG.wsUrl}/events`);
    
    ws.onopen = () => {
      ws.send(JSON.stringify({ action: 'subscribe', types: eventTypes }));
    };
    
    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      onEvent(data);
    };
    
    return ws;
  },

  async getHistory(params?: EventParams): Promise<Event[]> {
    const query = new URLSearchParams(params as any).toString();
    return http.get(`/events${query ? `?${query}` : ''}`);
  },
};

/**
 * Export all APIs as a single object for convenience
 */
export const AlphaPulseAPI = {
  auth,
  analysis,
  strategies,
  marketData,
  trading,
  templates,
  dataManagement,
  events,
  config: API_CONFIG,
  
  // Utility method to update configuration
  updateConfig(newConfig: Partial<ApiConfig>) {
    Object.assign(API_CONFIG, newConfig);
  },
};

// Re-export all types from types.ts
export type {
  AnalysisManifest,
  BacktestResult,
  BacktestParams,
  Strategy,
  Signal,
  SignalRequest,
  Position,
  Order,
  OrderRequest,
  OrderParams,
  MarketBar,
  ButtonTemplate,
  NotebookTemplate,
  Dataset,
  DatasetMetadata,
  Event,
  EventParams,
  AccountInfo,
  CompileResult,
} from './types';