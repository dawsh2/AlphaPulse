/**
 * Environment configuration
 */

export const env = {
  // API endpoints
  API_URL: import.meta.env.VITE_API_URL || 'http://localhost:8000/api',
  WS_URL: import.meta.env.VITE_WS_URL || 'ws://localhost:8080/ws',
  
  // Feature flags
  ENABLE_LIVE_TRADING: import.meta.env.VITE_ENABLE_LIVE_TRADING === 'true',
  ENABLE_PAPER_TRADING: import.meta.env.VITE_ENABLE_PAPER_TRADING !== 'false', // Default true
  ENABLE_BACKTESTING: import.meta.env.VITE_ENABLE_BACKTESTING !== 'false', // Default true
  ENABLE_AI_ASSISTANT: import.meta.env.VITE_ENABLE_AI_ASSISTANT === 'true',
  
  // App settings
  APP_NAME: import.meta.env.VITE_APP_NAME || 'AlphaPulse',
  APP_VERSION: import.meta.env.VITE_APP_VERSION || '1.0.0',
  
  // Development
  IS_DEV: import.meta.env.DEV,
  IS_PROD: import.meta.env.PROD,
  
  // Analytics
  ANALYTICS_ID: import.meta.env.VITE_ANALYTICS_ID || '',
  
  // Cache settings
  CACHE_TTL: parseInt(import.meta.env.VITE_CACHE_TTL || '3600'),
  MAX_CACHE_SIZE: parseInt(import.meta.env.VITE_MAX_CACHE_SIZE || '100'),
  
  // Rate limits
  MAX_REQUESTS_PER_MINUTE: parseInt(import.meta.env.VITE_MAX_REQUESTS_PER_MINUTE || '60'),
  MAX_BACKTESTS_PER_HOUR: parseInt(import.meta.env.VITE_MAX_BACKTESTS_PER_HOUR || '10'),
  
  // Data limits
  MAX_CHART_POINTS: parseInt(import.meta.env.VITE_MAX_CHART_POINTS || '10000'),
  MAX_WEBSOCKET_RECONNECTS: parseInt(import.meta.env.VITE_MAX_WEBSOCKET_RECONNECTS || '5'),
} as const;

// Validate required environment variables
export function validateEnv(): void {
  const required = ['API_URL', 'WS_URL'];
  const missing = required.filter(key => !env[key as keyof typeof env]);
  
  if (missing.length > 0) {
    console.warn(`Missing environment variables: ${missing.join(', ')}`);
  }
  
  if (env.IS_DEV) {
    console.log('Environment:', env);
  }
}