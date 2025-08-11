/**
 * ChartLayoutManager Component
 * Manages a tree-based layout system for unlimited chart tiling
 * Similar to DevelopLayoutManager but optimized for trading charts
 */

import React, { useState, useEffect, useRef } from 'react';
import { ChartWindow } from './ChartWindow';
import type { MarketData, ExchangeType } from '../../../services/exchanges';
import styles from '../../MonitorPage/MonitorPage.module.css';

// Chart configuration for each window
export interface ChartConfig {
  symbol: string;
  exchange: ExchangeType;
  timeframe: string;
  marketData: MarketData[];
  isLoadingData: boolean;
  livePrice: number | null;
}

// Window node containing a single chart
export type ChartWindowNode = {
  type: 'window';
  id: string;
  config: ChartConfig;
};

// Split node containing multiple children
export type SplitNode = {
  type: 'split';
  id: string;
  orientation: 'horizontal' | 'vertical';
  children: LayoutNode[];
  sizes: number[]; // Percentage for each child
};

export type LayoutNode = ChartWindowNode | SplitNode;

interface ChartLayoutManagerProps {
  layout: LayoutNode;
  onLayoutChange: (layout: LayoutNode) => void;
  onSymbolChange: (windowId: string, symbol: string) => void;
  onExchangeChange: (windowId: string, exchange: ExchangeType) => void;
  onTimeframeChange: (windowId: string, timeframe: string) => void;
  playbackControls?: {
    isPlaying: boolean;
    currentBar: number;
    playbackSpeed: number;
  };
}

export const ChartLayoutManager: React.FC<ChartLayoutManagerProps> = ({
  layout,
  onLayoutChange,
  onSymbolChange,
  onExchangeChange,
  onTimeframeChange,
  playbackControls
}) => {
  const [dragging, setDragging] = useState<{ splitId: string; index: number } | null>(null);
  const initializedWindowsRef = useRef<Set<string>>(new Set());

  // Effect to initialize new chart windows when they appear
  useEffect(() => {
    const initializeNewWindows = (node: LayoutNode) => {
      if (node.type === 'window') {
        // Check if this window needs initialization
        if (!initializedWindowsRef.current.has(node.id) && node.config.isLoadingData) {
          console.log(`[ChartLayoutManager] Auto-initializing new window: ${node.id}`);
          initializedWindowsRef.current.add(node.id);
          // Trigger initialization
          onSymbolChange(node.id, node.config.symbol);
        }
      } else if (node.type === 'split') {
        node.children.forEach(initializeNewWindows);
      }
    };

    initializeNewWindows(layout);
  }, [layout, onSymbolChange]);

  // Helper to update a specific window in the tree
  const updateWindow = (
    node: LayoutNode,
    windowId: string,
    updater: (window: ChartWindowNode) => ChartWindowNode
  ): LayoutNode => {
    if (node.type === 'window') {
      if (node.id === windowId) {
        return updater(node);
      }
      return node;
    } else {
      return {
        ...node,
        children: node.children.map(child => updateWindow(child, windowId, updater))
      };
    }
  };

  // Helper to split a window
  const splitWindow = (windowId: string, orientation: 'horizontal' | 'vertical') => {
    const splitNode = (node: LayoutNode): LayoutNode => {
      if (node.type === 'window' && node.id === windowId) {
        // Create new window with same config but new ID
        const newWindowId = `chart-window-${Date.now()}`;
        const newWindow: ChartWindowNode = {
          type: 'window',
          id: newWindowId,
          config: {
            ...node.config,
            marketData: [], // New window starts with empty data
            isLoadingData: true // Should show loading state initially
          }
        };

        // Create split node
        const splitNode: SplitNode = {
          type: 'split',
          id: `split-${Date.now()}`,
          orientation,
          children: [node, newWindow],
          sizes: [50, 50]
        };

        // New window will be auto-initialized by the useEffect when layout changes
        console.log(`[ChartLayoutManager] Created new chart window: ${newWindowId} for symbol: ${newWindow.config.symbol}`);

        return splitNode;
      } else if (node.type === 'split') {
        return {
          ...node,
          children: node.children.map(splitNode)
        };
      }
      return node;
    };

    onLayoutChange(splitNode(layout));
  };

  // Helper to close a window
  const closeWindow = (windowId: string) => {
    // Don't allow closing if it's the only window
    if (layout.type === 'window' && layout.id === windowId) {
      return; // Prevent closing the last window
    }
    
    const removeNode = (node: LayoutNode, parent?: SplitNode): LayoutNode | null => {
      if (node.type === 'window' && node.id === windowId) {
        return null;
      } else if (node.type === 'split') {
        const newChildren = node.children
          .map(child => removeNode(child, node))
          .filter(Boolean) as LayoutNode[];
        
        // If only one child remains, return it directly (flatten the tree)
        if (newChildren.length === 1) {
          return newChildren[0];
        } else if (newChildren.length === 0) {
          return null;
        }
        
        // Recalculate sizes
        const totalSize = 100;
        const newSizes = newChildren.map(() => totalSize / newChildren.length);
        
        return {
          ...node,
          children: newChildren,
          sizes: newSizes
        };
      }
      return node;
    };

    const newLayout = removeNode(layout);
    if (newLayout) {
      onLayoutChange(newLayout);
    }
  };

  // Handle splitter drag for resizing
  const handleSplitterDrag = (splitId: string, index: number, delta: number, total: number) => {
    const updateSizes = (node: LayoutNode): LayoutNode => {
      if (node.type === 'split' && node.id === splitId) {
        const newSizes = [...node.sizes];
        const deltaPercent = (delta / total) * 100;
        
        // Adjust the two adjacent panes
        newSizes[index] = Math.max(20, Math.min(80, newSizes[index] + deltaPercent));
        newSizes[index + 1] = Math.max(20, Math.min(80, newSizes[index + 1] - deltaPercent));
        
        return { ...node, sizes: newSizes };
      } else if (node.type === 'split') {
        return {
          ...node,
          children: node.children.map(updateSizes)
        };
      }
      return node;
    };

    onLayoutChange(updateSizes(layout));
  };

  // Render the layout tree recursively
  const renderNode = (node: LayoutNode): React.ReactElement => {
    if (node.type === 'window') {
      return (
        <ChartWindow
          key={node.id}
          windowId={node.id}
          config={node.config}
          onSymbolChange={(symbol) => onSymbolChange(node.id, symbol)}
          onExchangeChange={(exchange) => onExchangeChange(node.id, exchange)}
          onTimeframeChange={(timeframe) => onTimeframeChange(node.id, timeframe)}
          onSplit={(orientation) => splitWindow(node.id, orientation)}
          onClose={layout.type === 'split' ? () => closeWindow(node.id) : undefined}
          playbackControls={playbackControls}
        />
      );
    } else {
      // Split node
      const isHorizontal = node.orientation === 'horizontal';
      return (
        <div
          key={node.id}
          style={{
            display: 'flex',
            flexDirection: isHorizontal ? 'column' : 'row',
            width: '100%',
            height: '100%',
            flex: 1
          }}
        >
          {node.children.map((child, index) => (
            <React.Fragment key={index}>
              <div
                style={{
                  flex: `0 0 ${node.sizes[index]}%`,
                  display: 'flex',
                  overflow: 'hidden',
                  minWidth: isHorizontal ? 'auto' : '300px',
                  minHeight: isHorizontal ? '200px' : 'auto',
                  position: 'relative'
                }}
              >
                {renderNode(child)}
              </div>
              {index < node.children.length - 1 && (
                <div
                  className={`${styles.splitter} ${
                    isHorizontal ? styles.splitterHorizontal : styles.splitterVertical
                  }`}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    setDragging({ splitId: node.id, index });
                    
                    const startPos = isHorizontal ? e.clientY : e.clientX;
                    const containerSize = isHorizontal
                      ? e.currentTarget.parentElement!.clientHeight
                      : e.currentTarget.parentElement!.clientWidth;
                    
                    const handleMouseMove = (e: MouseEvent) => {
                      const currentPos = isHorizontal ? e.clientY : e.clientX;
                      const delta = currentPos - startPos;
                      handleSplitterDrag(node.id, index, delta, containerSize);
                    };
                    
                    const handleMouseUp = () => {
                      setDragging(null);
                      document.removeEventListener('mousemove', handleMouseMove);
                      document.removeEventListener('mouseup', handleMouseUp);
                    };
                    
                    document.addEventListener('mousemove', handleMouseMove);
                    document.addEventListener('mouseup', handleMouseUp);
                  }}
                  style={{
                    cursor: isHorizontal ? 'row-resize' : 'col-resize',
                    background: dragging?.splitId === node.id && dragging.index === index
                      ? 'var(--color-accent-primary)'
                      : 'var(--color-border-primary)'
                  }}
                />
              )}
            </React.Fragment>
          ))}
        </div>
      );
    }
  };

  return (
    <div className={styles.chartLayoutContainer}>
      {renderNode(layout)}
    </div>
  );
};