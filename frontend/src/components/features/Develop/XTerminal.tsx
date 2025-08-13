/**
 * XTerminal Component - Full terminal emulator with custom phosphor blue styling
 * Maintains exact visual compatibility with existing terminal design
 */
import React, { useEffect, useRef, useState } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { io } from 'socket.io-client';
import '@xterm/xterm/css/xterm.css';
import styles from '../../../pages/DevelopPage.module.css';

interface XTerminalProps {
  onCommand?: (command: string) => void;
  className?: string;
}

export const XTerminal: React.FC<XTerminalProps> = ({ 
  onCommand, 
  className 
}) => {
  const terminalRef = useRef<HTMLDivElement>(null);
  const terminal = useRef<Terminal | null>(null);
  const fitAddon = useRef<FitAddon | null>(null);
  const [isConnected, setIsConnected] = useState(false);

  useEffect(() => {
    if (!terminalRef.current) return;

    // Create terminal with custom theme matching your phosphor blue design
    terminal.current = new Terminal({
      theme: {
        background: 'transparent', // Let CSS handle the background
        foreground: '#00d4ff', // Phosphor blue text
        cursor: '#00d4ff', // Blue cursor
        cursorAccent: '#000000',
        selection: 'rgba(0, 212, 255, 0.3)', // Blue selection
        black: '#000000',
        red: '#ff6b6b',
        green: '#51cf66',
        yellow: '#ffd43b', 
        blue: '#339af0',
        magenta: '#f06292',
        cyan: '#00d4ff', // Phosphor blue
        white: '#ffffff',
        brightBlack: '#495057',
        brightRed: '#ff8a80',
        brightGreen: '#69f0ae',
        brightYellow: '#ffff8d',
        brightBlue: '#82b1ff',
        brightMagenta: '#ea80fc',
        brightCyan: '#00d4ff', // Phosphor blue
        brightWhite: '#ffffff'
      },
      fontFamily: 'IBM Plex Mono, SF Mono, Monaco, Consolas, Liberation Mono, Menlo, monospace',
      fontSize: 13,
      fontWeight: '500',
      lineHeight: 1.4,
      cursorBlink: true,
      cursorStyle: 'block', // Fat cursor like your design
      cursorWidth: 8, // Make it fat
      allowTransparency: true,
      convertEol: true,
      scrollback: 1000,
      rows: 30,
      cols: 80
    });

    // Add addons
    fitAddon.current = new FitAddon();
    const webLinksAddon = new WebLinksAddon();
    
    terminal.current.loadAddon(fitAddon.current);
    terminal.current.loadAddon(webLinksAddon);

    // Open terminal in the DOM element
    terminal.current.open(terminalRef.current);

    // Apply custom CSS styling to match your design
    const terminalElement = terminalRef.current.querySelector('.xterm-screen') as HTMLElement;
    if (terminalElement) {
      terminalElement.style.textShadow = '0 0 5px #00d4ff, 0 0 10px #00d4ff';
      terminalElement.style.fontWeight = '500';
    }

    // Handle resize
    const handleResize = () => {
      if (fitAddon.current && terminal.current) {
        fitAddon.current.fit();
      }
    };

    window.addEventListener('resize', handleResize);
    handleResize();

    // Connect to WebSocket terminal backend (we'll implement this)
    connectToTerminal();

    return () => {
      window.removeEventListener('resize', handleResize);
      if (terminal.current) {
        terminal.current.dispose();
      }
    };
  }, []);

  const connectToTerminal = async () => {
    if (!terminal.current) return;

    // Connect to WebSocket
    const socket = io('http://localhost:5001', {
      transports: ['websocket']
    });

    socket.on('connect', () => {
      console.log('✅ Connected to WebSocket server');
      
      // Request terminal session
      console.log('🔌 Requesting terminal session...');
      socket.emit('terminal_connect', {
        shell: '/bin/bash'
      });
    });

    socket.on('connect_error', (error) => {
      console.error('❌ WebSocket connection error:', error);
      if (terminal.current) {
        terminal.current.writeln(`\r\n\x1b[31mConnection Error: ${error.message}\x1b[0m\r\n`);
        terminal.current.writeln(`\x1b[33mTrying to connect to: http://localhost:5001\x1b[0m\r\n`);
        terminal.current.writeln(`\x1b[33mMake sure your backend is running with WebSocket support\x1b[0m\r\n`);
      }
    });

    socket.on('disconnect', (reason) => {
      console.log('🔌 WebSocket disconnected:', reason);
      setIsConnected(false);
    });

    socket.on('terminal_ready', (data) => {
      console.log('Terminal session ready:', data);
      setIsConnected(true);
      
      // Store session ID for later use
      (terminal.current as any)._sessionId = data.session_id;
      (terminal.current as any)._socket = socket;
    });

    socket.on('terminal_output', (data) => {
      if (terminal.current && data.data) {
        terminal.current.write(data.data);
      }
    });

    socket.on('terminal_error', (error) => {
      console.error('Terminal error:', error);
      if (terminal.current) {
        terminal.current.writeln(`\r\n\x1b[31mTerminal Error: ${error.error}\x1b[0m\r\n`);
      }
    });

    // Handle user input - send to PTY backend
    terminal.current.onData((data) => {
      const sessionId = (terminal.current as any)?._sessionId;
      const socket = (terminal.current as any)?._socket;
      
      if (sessionId && socket && socket.connected) {
        socket.emit('terminal_input', {
          session_id: sessionId,
          data: data
        });
      }
    });

    // Handle terminal resize
    const handleResize = () => {
      if (terminal.current && fitAddon.current) {
        fitAddon.current.fit();
        
        const sessionId = (terminal.current as any)?._sessionId;
        const socket = (terminal.current as any)?._socket;
        
        if (sessionId && socket && socket.connected) {
          socket.emit('terminal_resize', {
            session_id: sessionId,
            cols: terminal.current.cols,
            rows: terminal.current.rows
          });
        }
      }
    };

    // Listen for window resize
    window.addEventListener('resize', handleResize);
    
    // Cleanup on unmount
    return () => {
      const sessionId = (terminal.current as any)?._sessionId;
      if (sessionId && socket.connected) {
        socket.emit('terminal_disconnect', {
          session_id: sessionId
        });
      }
      socket.disconnect();
      window.removeEventListener('resize', handleResize);
    };
  };

  return (
    <div className={className} style={{ position: 'relative', height: '100%' }}>
      {/* Custom terminal container with your exact styling */}
      <div 
        className={styles.terminalContainer}
        style={{
          background: 'rgba(0, 0, 0, 0.95)',
          border: '3px solid #00d4ff',
          borderRadius: '8px',
          boxShadow: '0 0 20px rgba(0, 212, 255, 0.3), inset 0 0 20px rgba(0, 212, 255, 0.1)',
          padding: '20px',
          height: '100%',
          overflow: 'hidden'
        }}
      >
        <div
          ref={terminalRef}
          style={{ 
            height: '100%',
            width: '100%'
          }}
        />
        
        {/* Connection status indicator */}
        {!isConnected && (
          <div style={{
            position: 'absolute',
            top: '50%',
            left: '50%',
            transform: 'translate(-50%, -50%)',
            color: '#00d4ff',
            textShadow: '0 0 10px #00d4ff',
            fontSize: '14px'
          }}>
            Connecting to terminal...
          </div>
        )}
      </div>

      {/* Custom styles to override xterm.js defaults */}
      <style jsx>{`
        :global(.xterm-viewport) {
          background: transparent !important;
        }
        
        :global(.xterm-screen) {
          background: transparent !important;
        }
        
        :global(.xterm-cursor-layer .xterm-cursor-block) {
          background: #00d4ff !important;
          box-shadow: 0 0 5px #00d4ff, 0 0 10px #00d4ff !important;
        }
        
        :global(.xterm-rows) {
          color: #00d4ff !important;
          text-shadow: 0 0 5px #00d4ff, 0 0 10px #00d4ff !important;
          font-weight: 500 !important;
        }
        
        :global(.xterm-decoration-top) {
          display: none !important;
        }
        
        :global(.xterm-scroll-area) {
          background: transparent !important;
        }
      `}</style>
    </div>
  );
};