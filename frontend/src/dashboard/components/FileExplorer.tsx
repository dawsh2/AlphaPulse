import React, { useState, useEffect } from 'react';
import './FileExplorer.css';

interface FileNode {
  name: string;
  path: string;
  type: 'file' | 'directory';
  size?: number;
  modified?: Date;
  children?: FileNode[];
  collapsed?: boolean;
  level: number;
}

// File type to icon mapping
const getFileIcon = (filename: string, isDirectory: boolean): string => {
  if (isDirectory) return '📁';
  
  const ext = filename.toLowerCase().split('.').pop();
  switch (ext) {
    case 'ts': case 'tsx': return '🔷';
    case 'js': case 'jsx': return '🟨';
    case 'py': return '🐍';
    case 'rs': return '🦀';
    case 'json': return '📄';
    case 'css': return '🎨';
    case 'html': return '🌐';
    case 'md': return '📝';
    case 'yml': case 'yaml': return '⚙️';
    case 'toml': return '📋';
    case 'lock': return '🔒';
    case 'env': return '🔧';
    case 'git': return '🔀';
    case 'png': case 'jpg': case 'jpeg': case 'gif': case 'svg': return '🖼️';
    default: return '📄';
  }
};

// Get file type CSS class for syntax highlighting
const getFileTypeClass = (filename: string): string => {
  const ext = filename.toLowerCase().split('.').pop();
  switch (ext) {
    case 'ts': case 'tsx': return 'typescript';
    case 'js': case 'jsx': return 'javascript';
    case 'py': return 'python';
    case 'rs': return 'rust';
    case 'json': return 'json';
    case 'css': return 'css';
    case 'html': return 'html';
    case 'md': return 'markdown';
    case 'yml': case 'yaml': return 'yaml';
    case 'toml': return 'toml';
    default: return 'text';
  }
};

export function FileExplorer() {
  const [fileTree, setFileTree] = useState<FileNode[]>([]);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [showHiddenFiles, setShowHiddenFiles] = useState(false);

  // Mock file system data - in real app this would come from an API
  const mockFileSystem: FileNode[] = [
    {
      name: 'alphapulse',
      path: '/alphapulse',
      type: 'directory',
      level: 0,
      collapsed: false,
      children: [
        {
          name: 'frontend',
          path: '/alphapulse/frontend',
          type: 'directory',
          level: 1,
          collapsed: false,
          children: [
            {
              name: 'src',
              path: '/alphapulse/frontend/src',
              type: 'directory',
              level: 2,
              collapsed: false,
              children: [
                {
                  name: 'dashboard',
                  path: '/alphapulse/frontend/src/dashboard',
                  type: 'directory',
                  level: 3,
                  collapsed: false,
                  children: [
                    { name: 'App.tsx', path: '/alphapulse/frontend/src/dashboard/App.tsx', type: 'file', level: 4, size: 5432, modified: new Date() },
                    { name: 'main.tsx', path: '/alphapulse/frontend/src/dashboard/main.tsx', type: 'file', level: 4, size: 1234, modified: new Date() },
                    {
                      name: 'components',
                      path: '/alphapulse/frontend/src/dashboard/components',
                      type: 'directory',
                      level: 4,
                      collapsed: false,
                      children: [
                        { name: 'TodoList.tsx', path: '/alphapulse/frontend/src/dashboard/components/TodoList.tsx', type: 'file', level: 5, size: 8901, modified: new Date() },
                        { name: 'FileExplorer.tsx', path: '/alphapulse/frontend/src/dashboard/components/FileExplorer.tsx', type: 'file', level: 5, size: 6789, modified: new Date() },
                        { name: 'DataFlowMonitor.tsx', path: '/alphapulse/frontend/src/dashboard/components/DataFlowMonitor.tsx', type: 'file', level: 5, size: 4567, modified: new Date() },
                      ]
                    }
                  ]
                },
                {
                  name: 'components',
                  path: '/alphapulse/frontend/src/components',
                  type: 'directory',
                  level: 3,
                  collapsed: true,
                  children: [
                    { name: 'ui', path: '/alphapulse/frontend/src/components/ui', type: 'directory', level: 4, collapsed: true },
                    { name: 'features', path: '/alphapulse/frontend/src/components/features', type: 'directory', level: 4, collapsed: true },
                  ]
                }
              ]
            },
            { name: 'package.json', path: '/alphapulse/frontend/package.json', type: 'file', level: 2, size: 2345, modified: new Date() },
            { name: 'vite.config.ts', path: '/alphapulse/frontend/vite.config.ts', type: 'file', level: 2, size: 1567, modified: new Date() },
            { name: 'tsconfig.json', path: '/alphapulse/frontend/tsconfig.json', type: 'file', level: 2, size: 890, modified: new Date() },
          ]
        },
        {
          name: 'rust-services',
          path: '/alphapulse/rust-services',
          type: 'directory',
          level: 1,
          collapsed: false,
          children: [
            { name: 'Cargo.toml', path: '/alphapulse/rust-services/Cargo.toml', type: 'file', level: 2, size: 1234, modified: new Date() },
            { name: 'Cargo.lock', path: '/alphapulse/rust-services/Cargo.lock', type: 'file', level: 2, size: 45678, modified: new Date() },
            {
              name: 'api-server',
              path: '/alphapulse/rust-services/api-server',
              type: 'directory',
              level: 2,
              collapsed: true,
              children: [
                { name: 'src', path: '/alphapulse/rust-services/api-server/src', type: 'directory', level: 3, collapsed: true },
                { name: 'Cargo.toml', path: '/alphapulse/rust-services/api-server/Cargo.toml', type: 'file', level: 3, size: 567, modified: new Date() },
              ]
            },
            {
              name: 'collectors',
              path: '/alphapulse/rust-services/collectors',
              type: 'directory',
              level: 2,
              collapsed: true,
              children: [
                { name: 'src', path: '/alphapulse/rust-services/collectors/src', type: 'directory', level: 3, collapsed: true },
                { name: 'Cargo.toml', path: '/alphapulse/rust-services/collectors/Cargo.toml', type: 'file', level: 3, size: 456, modified: new Date() },
              ]
            }
          ]
        },
        {
          name: 'backend',
          path: '/alphapulse/backend',
          type: 'directory',
          level: 1,
          collapsed: true,
          children: [
            { name: 'app.py', path: '/alphapulse/backend/app.py', type: 'file', level: 2, size: 3456, modified: new Date() },
            { name: 'requirements.txt', path: '/alphapulse/backend/requirements.txt', type: 'file', level: 2, size: 789, modified: new Date() },
          ]
        },
        { name: 'README.md', path: '/alphapulse/README.md', type: 'file', level: 1, size: 2345, modified: new Date() },
        { name: 'docker-compose.yml', path: '/alphapulse/docker-compose.yml', type: 'file', level: 1, size: 1234, modified: new Date() },
        { name: '.gitignore', path: '/alphapulse/.gitignore', type: 'file', level: 1, size: 567, modified: new Date() },
        { name: '.env.example', path: '/alphapulse/.env.example', type: 'file', level: 1, size: 234, modified: new Date() },
      ]
    }
  ];

  useEffect(() => {
    // Simulate API call
    const loadFileTree = async () => {
      setLoading(true);
      try {
        // In real app, this would be an API call
        await new Promise(resolve => setTimeout(resolve, 500));
        setFileTree(mockFileSystem);
        setError(null);
      } catch (err) {
        setError('Failed to load file tree');
      } finally {
        setLoading(false);
      }
    };

    loadFileTree();
  }, []);

  const toggleDirectory = (path: string) => {
    const updateNode = (nodes: FileNode[]): FileNode[] => {
      return nodes.map(node => {
        if (node.path === path && node.type === 'directory') {
          return { ...node, collapsed: !node.collapsed };
        }
        if (node.children) {
          return { ...node, children: updateNode(node.children) };
        }
        return node;
      });
    };

    setFileTree(updateNode(fileTree));
  };

  const flattenTree = (nodes: FileNode[], showHidden: boolean = false): FileNode[] => {
    const result: FileNode[] = [];
    
    const traverse = (nodes: FileNode[]) => {
      nodes.forEach(node => {
        // Filter hidden files
        if (!showHidden && node.name.startsWith('.') && node.level > 0) {
          return;
        }
        
        result.push(node);
        
        if (node.children && !node.collapsed) {
          traverse(node.children);
        }
      });
    };
    
    traverse(nodes);
    return result;
  };

  const filteredTree = flattenTree(fileTree, showHiddenFiles).filter(node => {
    if (!searchTerm) return true;
    return node.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
           node.path.toLowerCase().includes(searchTerm.toLowerCase());
  });

  const formatFileSize = (bytes: number): string => {
    const sizes = ['B', 'KB', 'MB', 'GB'];
    if (bytes === 0) return '0 B';
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i];
  };

  const getStats = () => {
    const allFiles = flattenTree(fileTree, true);
    const files = allFiles.filter(n => n.type === 'file');
    const directories = allFiles.filter(n => n.type === 'directory');
    const totalSize = files.reduce((sum, file) => sum + (file.size || 0), 0);
    
    return {
      files: files.length,
      directories: directories.length,
      totalSize: formatFileSize(totalSize)
    };
  };

  const stats = getStats();

  if (loading) {
    return (
      <div className="file-explorer loading">
        <div className="loading-spinner">
          <div className="spinner"></div>
          <span>Loading project files...</span>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="file-explorer error">
        <div className="error-message">
          <span className="error-icon">⚠️</span>
          <span>{error}</span>
          <button onClick={() => window.location.reload()}>Retry</button>
        </div>
      </div>
    );
  }

  return (
    <div className="file-explorer">
      <div className="file-explorer-header">
        <h2>Project Explorer</h2>
        <div className="file-stats">
          <span className="stat">{stats.directories} folders</span>
          <span className="stat">{stats.files} files</span>
          <span className="stat">{stats.totalSize}</span>
        </div>
      </div>

      <div className="file-explorer-controls">
        <div className="search-bar">
          <input
            type="text"
            placeholder="Search files..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="search-input"
          />
          <span className="search-icon">🔍</span>
        </div>
        <div className="file-options">
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={showHiddenFiles}
              onChange={(e) => setShowHiddenFiles(e.target.checked)}
            />
            <span>Show hidden files</span>
          </label>
        </div>
      </div>

      <div className="file-tree">
        {filteredTree.length === 0 ? (
          <div className="empty-state">
            {searchTerm ? `No files match "${searchTerm}"` : 'No files found'}
          </div>
        ) : (
          filteredTree.map((node) => (
            <div
              key={node.path}
              className={`file-node ${node.type} ${getFileTypeClass(node.name)} ${selectedPath === node.path ? 'selected' : ''}`}
              style={{ paddingLeft: `${node.level * 20 + 8}px` }}
              onClick={() => setSelectedPath(node.path)}
            >
              <div className="file-node-content">
                {node.type === 'directory' && (
                  <button
                    className={`fold-button ${node.collapsed ? 'collapsed' : ''}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      toggleDirectory(node.path);
                    }}
                  >
                    {node.collapsed ? '▶' : '▼'}
                  </button>
                )}
                {node.type === 'file' && <span className="file-spacer">•</span>}
                
                <span className="file-icon">
                  {getFileIcon(node.name, node.type === 'directory')}
                </span>
                
                <span className="file-name">{node.name}</span>
                
                {node.type === 'file' && node.size && (
                  <span className="file-size">{formatFileSize(node.size)}</span>
                )}
                
                {node.modified && (
                  <span className="file-modified">
                    {node.modified.toLocaleDateString()}
                  </span>
                )}
              </div>
              
              {selectedPath === node.path && (
                <div className="file-details">
                  <div className="detail-row">
                    <span className="detail-label">Path:</span>
                    <span className="detail-value">{node.path}</span>
                  </div>
                  {node.size && (
                    <div className="detail-row">
                      <span className="detail-label">Size:</span>
                      <span className="detail-value">{formatFileSize(node.size)}</span>
                    </div>
                  )}
                  {node.modified && (
                    <div className="detail-row">
                      <span className="detail-label">Modified:</span>
                      <span className="detail-value">{node.modified.toLocaleString()}</span>
                    </div>
                  )}
                </div>
              )}
            </div>
          ))
        )}
      </div>

      <div className="file-explorer-footer">
        <div className="breadcrumb">
          {selectedPath && (
            <>
              <span className="breadcrumb-label">Selected:</span>
              <span className="breadcrumb-path">{selectedPath}</span>
            </>
          )}
        </div>
      </div>
    </div>
  );
}