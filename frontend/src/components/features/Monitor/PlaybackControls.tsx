import React, { useState } from 'react';
import styles from '../../MonitorPage/MonitorPage.module.css';

type PlaybackSpeed = 1 | 2 | 5 | 10;

interface PlaybackControlsProps {
  isPlaying: boolean;
  playbackSpeed: PlaybackSpeed;
  currentBar: number;
  maxBars: number;
  onTogglePlay: () => void;
  onSkipBackward: () => void;
  onSkipForward: () => void;
  onSpeedChange: (speed: PlaybackSpeed) => void;
  styles: Record<string, string>;
}

const PlaybackControls: React.FC<PlaybackControlsProps> = ({
  isPlaying,
  playbackSpeed,
  currentBar,
  maxBars,
  onTogglePlay,
  onSkipBackward,
  onSkipForward,
  onSpeedChange,
  styles
}) => {
  const [speedDropdownOpen, setSpeedDropdownOpen] = useState(false);

  return (
    <div className={styles.controlGroup}>
      <label className={styles.controlLabel}>Replay:</label>
      <div className={styles.replayControls}>
        <button className={styles.replayBtn} onClick={onSkipBackward}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <polygon points="11 19 2 12 11 5 11 19"></polygon>
            <polygon points="22 19 13 12 22 5 22 19"></polygon>
          </svg>
        </button>
        <button
          className={`${styles.replayBtn} ${isPlaying ? styles.active : ''}`}
          onClick={onTogglePlay}
        >
          {isPlaying ? (
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <rect x="6" y="4" width="4" height="16"></rect>
              <rect x="14" y="4" width="4" height="16"></rect>
            </svg>
          ) : (
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <polygon points="5 3 19 12 5 21 5 3"></polygon>
            </svg>
          )}
        </button>
        <button className={styles.replayBtn} onClick={onSkipForward}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <polygon points="13 19 22 12 13 5 13 19"></polygon>
            <polygon points="2 19 11 12 2 5 2 19"></polygon>
          </svg>
        </button>
        <div 
          className={styles.dropdownWrapper}
          onMouseEnter={() => setSpeedDropdownOpen(true)}
          onMouseLeave={() => setSpeedDropdownOpen(false)}
        >
          <button className={styles.dropdownButton}>
            {playbackSpeed}x
            <span style={{ marginLeft: '8px' }}>▼</span>
          </button>
          {speedDropdownOpen && (
            <div className={styles.dropdownMenu}>
              {[1, 2, 5, 10].map((speed) => (
                <button
                  key={speed}
                  className={`${styles.dropdownOption} ${playbackSpeed === speed ? styles.active : ''}`}
                  onClick={() => onSpeedChange(speed as PlaybackSpeed)}
                >
                  {speed}x
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default PlaybackControls;
export type { PlaybackSpeed };