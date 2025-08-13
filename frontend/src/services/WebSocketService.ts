// WebSocket service for real-time market data streaming
import { EventEmitter } from 'events';

export interface SubscriptionRequest {
  type: 'subscribe' | 'unsubscribe';
  channels: string[];
  symbols: string[];
}

export interface MarketDataUpdate {
  type: 'trade' | 'orderbook';
  channel: string;
  symbol: string;
  data: any;
  timestamp: number;
}

export interface Trade {
  timestamp: number;
  price: number;
  volume: number;
  side?: string;
  trade_id?: string;
  symbol: string;
  exchange: string;
}

export interface OrderBook {
  bids: Array<[number, number]>; // [price, volume]
  asks: Array<[number, number]>; // [price, volume]
  timestamp: number;
}

class WebSocketService extends EventEmitter {
  private ws: WebSocket | null = null;
  private url: string;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private subscriptions: SubscriptionRequest | null = null;
  private isConnecting = false;

  constructor(url: string = 'ws://localhost:3001/ws') {
    super();
    this.url = url;
  }

  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        resolve();
        return;
      }

      if (this.isConnecting) {
        // Wait for existing connection attempt
        const checkConnection = setInterval(() => {
          if (this.ws?.readyState === WebSocket.OPEN) {
            clearInterval(checkConnection);
            resolve();
          } else if (!this.isConnecting) {
            clearInterval(checkConnection);
            reject(new Error('Connection failed'));
          }
        }, 100);
        return;
      }

      this.isConnecting = true;
      console.log('Connecting to WebSocket:', this.url);

      try {
        this.ws = new WebSocket(this.url);

        this.ws.onopen = () => {
          console.log('WebSocket connected');
          this.reconnectAttempts = 0;
          this.isConnecting = false;
          this.emit('connected');
          
          // Restore subscriptions if any
          if (this.subscriptions) {
            this.sendMessage(this.subscriptions);
          }
          
          resolve();
        };

        this.ws.onmessage = (event) => {
          try {
            const update: MarketDataUpdate = JSON.parse(event.data);
            this.handleUpdate(update);
          } catch (error) {
            console.error('Failed to parse WebSocket message:', error);
          }
        };

        this.ws.onerror = (error) => {
          console.error('WebSocket error:', error);
          this.isConnecting = false;
          this.emit('error', error);
          reject(error);
        };

        this.ws.onclose = () => {
          console.log('WebSocket disconnected');
          this.isConnecting = false;
          this.emit('disconnected');
          this.attemptReconnect();
        };
      } catch (error) {
        this.isConnecting = false;
        reject(error);
      }
    });
  }

  private handleUpdate(update: MarketDataUpdate) {
    // Emit specific events based on update type
    switch (update.type) {
      case 'trade':
        this.emit('trade', update.symbol, update.data as Trade);
        break;
      case 'orderbook':
        this.emit('orderbook', update.symbol, update.data as OrderBook);
        break;
      default:
        console.warn('Unknown update type:', update.type);
    }
    
    // Also emit a general update event
    this.emit('update', update);
  }

  subscribe(channels: string[], symbols: string[]) {
    const request: SubscriptionRequest = {
      type: 'subscribe',
      channels,
      symbols
    };
    
    this.subscriptions = request;
    this.sendMessage(request);
  }

  unsubscribe() {
    const request: SubscriptionRequest = {
      type: 'unsubscribe',
      channels: [],
      symbols: []
    };
    
    this.subscriptions = null;
    this.sendMessage(request);
  }

  private sendMessage(message: any) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
      console.log('Sent message:', message);
    } else {
      console.warn('WebSocket not connected, cannot send message');
    }
  }

  private attemptReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('Max reconnection attempts reached');
      this.emit('reconnect_failed');
      return;
    }

    this.reconnectAttempts++;
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);
    
    console.log(`Attempting reconnect #${this.reconnectAttempts} in ${delay}ms`);
    
    setTimeout(() => {
      this.connect().catch(error => {
        console.error('Reconnection failed:', error);
      });
    }, delay);
  }

  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.subscriptions = null;
    this.reconnectAttempts = 0;
  }

  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }
}

// Export singleton instance
const wsService = new WebSocketService();
export default wsService;