/**
 * Generic WebSocket hook for real-time connections
 */

import { useState, useEffect, useRef, useCallback } from 'react';

interface UseWebSocketOptions {
  autoConnect?: boolean;
  reconnect?: boolean;
  reconnectInterval?: number;
  reconnectAttempts?: number;
  onOpen?: (event: Event) => void;
  onClose?: (event: CloseEvent) => void;
  onError?: (event: Event) => void;
  onMessage?: (data: any) => void;
}

export function useWebSocket(
  url: string,
  options: UseWebSocketOptions = {}
) {
  const {
    autoConnect = true,
    reconnect = true,
    reconnectInterval = 5000,
    reconnectAttempts = 5,
    onOpen,
    onClose,
    onError,
    onMessage,
  } = options;

  const [connected, setConnected] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [lastMessage, setLastMessage] = useState<any>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectCountRef = useRef(0);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout>();

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    setConnecting(true);
    setError(null);

    try {
      const ws = new WebSocket(url);

      ws.onopen = (event) => {
        setConnected(true);
        setConnecting(false);
        reconnectCountRef.current = 0;
        onOpen?.(event);
      };

      ws.onclose = (event) => {
        setConnected(false);
        setConnecting(false);
        wsRef.current = null;
        onClose?.(event);

        // Attempt reconnection if enabled
        if (
          reconnect &&
          !event.wasClean &&
          reconnectCountRef.current < reconnectAttempts
        ) {
          reconnectCountRef.current++;
          reconnectTimeoutRef.current = setTimeout(() => {
            connect();
          }, reconnectInterval);
        }
      };

      ws.onerror = (event) => {
        setError(new Error('WebSocket connection error'));
        setConnecting(false);
        onError?.(event);
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          setLastMessage(data);
          onMessage?.(data);
        } catch (err) {
          console.error('Failed to parse WebSocket message:', err);
        }
      };

      wsRef.current = ws;
    } catch (err) {
      setError(err as Error);
      setConnecting(false);
    }
  }, [url, reconnect, reconnectInterval, reconnectAttempts, onOpen, onClose, onError, onMessage]);

  const disconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
    }

    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    setConnected(false);
    setConnecting(false);
    reconnectCountRef.current = 0;
  }, []);

  const send = useCallback((data: any) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      const message = typeof data === 'string' ? data : JSON.stringify(data);
      wsRef.current.send(message);
      return true;
    }
    return false;
  }, []);

  // Auto-connect on mount if enabled
  useEffect(() => {
    if (autoConnect) {
      connect();
    }

    return () => {
      disconnect();
    };
  }, [url, autoConnect]);

  return {
    connected,
    connecting,
    error,
    lastMessage,
    connect,
    disconnect,
    send,
  };
}