/**
 * DevelopLayoutManager Component
 * Manages a tree-based layout system for unlimited window tiling
 */

import React, { useState, useRef } from 'react';
import { DevelopWindow } from './DevelopWindow';
import styles from '../../../pages/DevelopPage.module.css';

export interface UnifiedTab {
  id: string;
  name: string;
  type: 'editor' | 'terminal';
  content?: string;
  language?: string;
  terminalContent?: string[];
  currentInput?: string;
  cwd?: string;
}

export type WindowNode = {
  type: 'window';
  id: string;
  tabs: UnifiedTab[];
  activeTab: string;
};

export type SplitNode = {
  type: 'split';
  id: string;
  orientation: 'horizontal' | 'vertical';
  children: LayoutNode[];
  sizes: number[]; // Percentage for each child
};

export type LayoutNode = WindowNode | SplitNode;

interface DevelopLayoutManagerProps {
  layout: LayoutNode;
  onLayoutChange: (layout: LayoutNode) => void;
  onOpenFile: (filePath: string, fileName: string, windowId: string) => void;
  onSaveFile: () => void;
}

export const DevelopLayoutManager: React.FC<DevelopLayoutManagerProps> = ({
  layout,
  onLayoutChange,
  onOpenFile,
  onSaveFile
}) => {
  const [dragging, setDragging] = useState<{ splitId: string; index: number } | null>(null);

  // Helper to update a specific window in the tree
  const updateWindow = (node: LayoutNode, windowId: string, updater: (window: WindowNode) => WindowNode): LayoutNode => {
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
        // Create new window with a terminal tab
        const terminalTabId = `terminal-${Date.now()}`;
        const newWindow: WindowNode = {
          type: 'window',
          id: `window-${Date.now()}`,
          tabs: [{
            id: terminalTabId,
            name: '~/strategies',
            type: 'terminal',
            terminalContent: [],
            currentInput: '',
            cwd: '~/strategies'
          }],
          activeTab: terminalTabId
        };

        // Create split node
        const splitNode: SplitNode = {
          type: 'split',
          id: `split-${Date.now()}`,
          orientation,
          children: [node, newWindow],
          sizes: [50, 50]
        };

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

  // Handle splitter drag
  const handleSplitterDrag = (splitId: string, index: number, delta: number, total: number) => {
    const updateSizes = (node: LayoutNode): LayoutNode => {
      if (node.type === 'split' && node.id === splitId) {
        const newSizes = [...node.sizes];
        const deltaPercent = (delta / total) * 100;
        
        // Adjust the two adjacent panes
        newSizes[index] = Math.max(10, Math.min(90, newSizes[index] + deltaPercent));
        newSizes[index + 1] = Math.max(10, Math.min(90, newSizes[index + 1] - deltaPercent));
        
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
  const renderNode = (node: LayoutNode): JSX.Element => {
    if (node.type === 'window') {
      return (
        <DevelopWindow
          key={node.id}
          tabs={node.tabs}
          activeTab={node.activeTab}
          setTabs={(tabs) => {
            onLayoutChange(updateWindow(layout, node.id, w => ({ ...w, tabs })));
          }}
          setActiveTab={(tabId) => {
            onLayoutChange(updateWindow(layout, node.id, w => ({ ...w, activeTab: tabId })));
          }}
          onNewTab={() => {
            const newTab: UnifiedTab = {
              id: `terminal-${Date.now()}`,
              name: '~/strategies',
              type: 'terminal',
              terminalContent: [],
              currentInput: '',
              cwd: '~/strategies'
            };
            onLayoutChange(updateWindow(layout, node.id, w => ({
              ...w,
              tabs: [...w.tabs, newTab],
              activeTab: newTab.id
            })));
          }}
          onCloseTab={(tabId, e) => {
            e.stopPropagation();
            onLayoutChange(updateWindow(layout, node.id, w => {
              const newTabs = w.tabs.filter(tab => tab.id !== tabId);
              const newActiveTab = w.activeTab === tabId && newTabs.length > 0 
                ? newTabs[0].id 
                : w.activeTab;
              return { ...w, tabs: newTabs, activeTab: newActiveTab };
            }));
          }}
          onSaveFile={onSaveFile}
          onSplitWindow={(orientation) => splitWindow(node.id, orientation)}
          isSplit={true}
          onCloseWindow={layout.type === 'split' ? () => closeWindow(node.id) : undefined}
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
                  minWidth: isHorizontal ? 'auto' : '200px',
                  minHeight: isHorizontal ? '100px' : 'auto'
                }}
              >
                {renderNode(child)}
              </div>
              {index < node.children.length - 1 && (
                <div
                  className={`${styles.splitter} ${isHorizontal ? styles.splitterHorizontal : styles.splitterVertical}`}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    setDragging({ splitId: node.id, index });
                    
                    const startPos = isHorizontal ? e.clientY : e.clientX;
                    const container = e.currentTarget.parentElement;
                    const totalSize = isHorizontal 
                      ? container?.clientHeight || 0
                      : container?.clientWidth || 0;
                    
                    const handleMouseMove = (e: MouseEvent) => {
                      const currentPos = isHorizontal ? e.clientY : e.clientX;
                      const delta = currentPos - startPos;
                      handleSplitterDrag(node.id, index, delta, totalSize);
                    };
                    
                    const handleMouseUp = () => {
                      setDragging(null);
                      document.removeEventListener('mousemove', handleMouseMove);
                      document.removeEventListener('mouseup', handleMouseUp);
                    };
                    
                    document.addEventListener('mousemove', handleMouseMove);
                    document.addEventListener('mouseup', handleMouseUp);
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
    <div style={{ width: '100%', height: '100%', display: 'flex' }}>
      {renderNode(layout)}
    </div>
  );
};