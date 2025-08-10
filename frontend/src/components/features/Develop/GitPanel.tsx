/**
 * Git Panel component for development environment
 */

import React, { useState, useCallback } from 'react';
import styles from './Develop.module.css';

interface GitFile {
  path: string;
  status: 'modified' | 'added' | 'deleted' | 'renamed' | 'untracked';
  staged: boolean;
}

interface GitCommit {
  hash: string;
  author: string;
  date: Date;
  message: string;
}

interface GitPanelProps {
  files?: GitFile[];
  commits?: GitCommit[];
  currentBranch?: string;
  onStageFile?: (path: string) => void;
  onUnstageFile?: (path: string) => void;
  onCommit?: (message: string) => void;
  onPush?: () => void;
  onPull?: () => void;
  onBranchChange?: (branch: string) => void;
}

export const GitPanel: React.FC<GitPanelProps> = ({
  files = [],
  commits = [],
  currentBranch = 'main',
  onStageFile,
  onUnstageFile,
  onCommit,
  onPush,
  onPull,
  onBranchChange,
}) => {
  const [activeTab, setActiveTab] = useState<'changes' | 'history'>('changes');
  const [commitMessage, setCommitMessage] = useState('');
  const [showBranchSelector, setShowBranchSelector] = useState(false);

  const stagedFiles = files.filter(f => f.staged);
  const unstagedFiles = files.filter(f => !f.staged);

  const getStatusIcon = (status: GitFile['status']) => {
    switch (status) {
      case 'modified': return 'M';
      case 'added': return 'A';
      case 'deleted': return 'D';
      case 'renamed': return 'R';
      case 'untracked': return 'U';
      default: return '?';
    }
  };

  const getStatusColor = (status: GitFile['status']) => {
    switch (status) {
      case 'modified': return styles.modified;
      case 'added': return styles.added;
      case 'deleted': return styles.deleted;
      case 'renamed': return styles.renamed;
      case 'untracked': return styles.untracked;
      default: return '';
    }
  };

  const handleCommit = useCallback(() => {
    if (commitMessage.trim() && stagedFiles.length > 0) {
      onCommit?.(commitMessage);
      setCommitMessage('');
    }
  }, [commitMessage, stagedFiles, onCommit]);

  const formatDate = (date: Date) => {
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const hours = Math.floor(diff / (1000 * 60 * 60));
    
    if (hours < 1) return 'just now';
    if (hours < 24) return `${hours} hours ago`;
    if (hours < 48) return 'yesterday';
    return date.toLocaleDateString();
  };

  return (
    <div className={styles.gitPanel}>
      <div className={styles.gitHeader}>
        <div className={styles.gitBranch}>
          <button
            className={styles.branchButton}
            onClick={() => setShowBranchSelector(!showBranchSelector)}
          >
            🌿 {currentBranch}
          </button>
          {showBranchSelector && (
            <div className={styles.branchSelector}>
              <input
                type="text"
                placeholder="Switch branch..."
                className={styles.branchInput}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    onBranchChange?.(e.currentTarget.value);
                    setShowBranchSelector(false);
                  }
                }}
                autoFocus
              />
            </div>
          )}
        </div>
        
        <div className={styles.gitActions}>
          <button
            className={styles.gitButton}
            onClick={onPull}
            title="Pull"
          >
            ⬇️
          </button>
          <button
            className={styles.gitButton}
            onClick={onPush}
            title="Push"
          >
            ⬆️
          </button>
        </div>
      </div>

      <div className={styles.gitTabs}>
        <button
          className={`${styles.gitTab} ${activeTab === 'changes' ? styles.active : ''}`}
          onClick={() => setActiveTab('changes')}
        >
          Changes {files.length > 0 && `(${files.length})`}
        </button>
        <button
          className={`${styles.gitTab} ${activeTab === 'history' ? styles.active : ''}`}
          onClick={() => setActiveTab('history')}
        >
          History
        </button>
      </div>

      {activeTab === 'changes' && (
        <div className={styles.gitChanges}>
          {stagedFiles.length > 0 && (
            <div className={styles.fileSection}>
              <div className={styles.sectionHeader}>
                Staged Changes ({stagedFiles.length})
              </div>
              {stagedFiles.map(file => (
                <div key={file.path} className={styles.gitFile}>
                  <span className={`${styles.fileStatus} ${getStatusColor(file.status)}`}>
                    {getStatusIcon(file.status)}
                  </span>
                  <span className={styles.fileName}>{file.path}</span>
                  <button
                    className={styles.unstageButton}
                    onClick={() => onUnstageFile?.(file.path)}
                    title="Unstage"
                  >
                    −
                  </button>
                </div>
              ))}
            </div>
          )}

          {unstagedFiles.length > 0 && (
            <div className={styles.fileSection}>
              <div className={styles.sectionHeader}>
                Changes ({unstagedFiles.length})
              </div>
              {unstagedFiles.map(file => (
                <div key={file.path} className={styles.gitFile}>
                  <span className={`${styles.fileStatus} ${getStatusColor(file.status)}`}>
                    {getStatusIcon(file.status)}
                  </span>
                  <span className={styles.fileName}>{file.path}</span>
                  <button
                    className={styles.stageButton}
                    onClick={() => onStageFile?.(file.path)}
                    title="Stage"
                  >
                    +
                  </button>
                </div>
              ))}
            </div>
          )}

          {files.length === 0 && (
            <div className={styles.noChanges}>
              No changes to commit
            </div>
          )}

          {stagedFiles.length > 0 && (
            <div className={styles.commitSection}>
              <textarea
                className={styles.commitMessage}
                placeholder="Commit message..."
                value={commitMessage}
                onChange={(e) => setCommitMessage(e.target.value)}
                rows={3}
              />
              <button
                className={styles.commitButton}
                onClick={handleCommit}
                disabled={!commitMessage.trim()}
              >
                Commit {stagedFiles.length} file{stagedFiles.length !== 1 ? 's' : ''}
              </button>
            </div>
          )}
        </div>
      )}

      {activeTab === 'history' && (
        <div className={styles.gitHistory}>
          {commits.length > 0 ? (
            commits.map(commit => (
              <div key={commit.hash} className={styles.commit}>
                <div className={styles.commitHeader}>
                  <span className={styles.commitHash}>
                    {commit.hash.substring(0, 7)}
                  </span>
                  <span className={styles.commitDate}>
                    {formatDate(commit.date)}
                  </span>
                </div>
                <div className={styles.commitMessage}>
                  {commit.message}
                </div>
                <div className={styles.commitAuthor}>
                  {commit.author}
                </div>
              </div>
            ))
          ) : (
            <div className={styles.noCommits}>
              No commits yet
            </div>
          )}
        </div>
      )}
    </div>
  );
};