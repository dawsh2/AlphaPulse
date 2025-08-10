/**
 * File Explorer component for development environment
 */

import React, { useState, useCallback } from 'react';
import styles from './Develop.module.css';

export interface FileItem {
  path: string;
  name: string;
  type: 'file' | 'folder';
  children?: FileItem[];
  content?: string;
  language?: string;
}

interface FileExplorerProps {
  files: FileItem[];
  selectedFile?: string;
  onFileSelect: (file: FileItem) => void;
  onFileCreate: (path: string, type: 'file' | 'folder') => void;
  onFileDelete: (path: string) => void;
  onFileRename: (oldPath: string, newPath: string) => void;
}

export const FileExplorer: React.FC<FileExplorerProps> = ({
  files,
  selectedFile,
  onFileSelect,
  onFileCreate,
  onFileDelete,
  onFileRename,
}) => {
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(
    new Set(['examples/', 'strategies/', 'indicators/'])
  );
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    item: FileItem;
  } | null>(null);
  const [renamingFile, setRenamingFile] = useState<string | null>(null);
  const [newName, setNewName] = useState('');

  const toggleFolder = useCallback((path: string) => {
    setExpandedFolders(prev => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

  const handleContextMenu = useCallback((e: React.MouseEvent, item: FileItem) => {
    e.preventDefault();
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      item,
    });
  }, []);

  const handleRename = useCallback((item: FileItem) => {
    setRenamingFile(item.path);
    setNewName(item.name);
    setContextMenu(null);
  }, []);

  const confirmRename = useCallback(() => {
    if (renamingFile && newName) {
      const newPath = renamingFile.replace(/[^/]+$/, newName);
      onFileRename(renamingFile, newPath);
      setRenamingFile(null);
      setNewName('');
    }
  }, [renamingFile, newName, onFileRename]);

  const renderFileTree = (items: FileItem[], depth: number = 0): React.ReactNode => {
    return items.map(item => (
      <div key={item.path}>
        <div
          className={`${styles.fileItem} ${
            selectedFile === item.path ? styles.selected : ''
          }`}
          style={{ paddingLeft: `${depth * 16 + 8}px` }}
          onClick={() => {
            if (item.type === 'folder') {
              toggleFolder(item.path);
            } else {
              onFileSelect(item);
            }
          }}
          onContextMenu={(e) => handleContextMenu(e, item)}
        >
          <span className={styles.fileIcon}>
            {item.type === 'folder' ? (
              expandedFolders.has(item.path) ? '📂' : '📁'
            ) : (
              getFileIcon(item.name)
            )}
          </span>
          {renamingFile === item.path ? (
            <input
              className={styles.renameInput}
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              onBlur={confirmRename}
              onKeyDown={(e) => {
                if (e.key === 'Enter') confirmRename();
                if (e.key === 'Escape') {
                  setRenamingFile(null);
                  setNewName('');
                }
              }}
              autoFocus
            />
          ) : (
            <span className={styles.fileName}>{item.name}</span>
          )}
        </div>
        {item.type === 'folder' &&
          item.children &&
          expandedFolders.has(item.path) &&
          renderFileTree(item.children, depth + 1)}
      </div>
    ));
  };

  return (
    <div className={styles.fileExplorer}>
      <div className={styles.explorerHeader}>
        <span>Explorer</span>
        <div className={styles.explorerActions}>
          <button
            onClick={() => onFileCreate('/', 'file')}
            title="New File"
            className={styles.iconButton}
          >
            📄
          </button>
          <button
            onClick={() => onFileCreate('/', 'folder')}
            title="New Folder"
            className={styles.iconButton}
          >
            📁
          </button>
        </div>
      </div>
      
      <div className={styles.fileTree}>
        {renderFileTree(files)}
      </div>

      {contextMenu && (
        <>
          <div
            className={styles.contextMenuOverlay}
            onClick={() => setContextMenu(null)}
          />
          <div
            className={styles.contextMenu}
            style={{ left: contextMenu.x, top: contextMenu.y }}
          >
            <button onClick={() => handleRename(contextMenu.item)}>
              Rename
            </button>
            <button onClick={() => {
              onFileDelete(contextMenu.item.path);
              setContextMenu(null);
            }}>
              Delete
            </button>
            {contextMenu.item.type === 'folder' && (
              <>
                <button onClick={() => {
                  onFileCreate(contextMenu.item.path, 'file');
                  setContextMenu(null);
                }}>
                  New File
                </button>
                <button onClick={() => {
                  onFileCreate(contextMenu.item.path, 'folder');
                  setContextMenu(null);
                }}>
                  New Folder
                </button>
              </>
            )}
          </div>
        </>
      )}
    </div>
  );
};

function getFileIcon(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'py': return '🐍';
    case 'js':
    case 'ts':
    case 'tsx': return '📜';
    case 'json': return '📋';
    case 'md': return '📝';
    case 'yaml':
    case 'yml': return '⚙️';
    case 'ipynb': return '📓';
    default: return '📄';
  }
}