import React, { useState } from 'react';
import { ChatIcon } from './Icons';
import styles from './GlobalAIChat.module.css';

export const GlobalAIChat: React.FC = () => {
  const [isOpen, setIsOpen] = useState(false);

  const toggleChat = () => {
    setIsOpen(!isOpen);
  };

  return (
    <>
      <button 
        className={styles.chatButton} 
        onClick={toggleChat}
        title="AI Assistant"
      >
        <ChatIcon size={24} />
      </button>
      
      {isOpen && (
        <div className={styles.chatOverlay} onClick={toggleChat}>
          <div 
            className={styles.chatContainer} 
            onClick={(e) => e.stopPropagation()}
          >
            <div className={styles.chatHeader}>
              <h3>AI Assistant</h3>
              <button 
                className={styles.closeButton} 
                onClick={toggleChat}
              >
                ×
              </button>
            </div>
            <div className={styles.chatContent}>
              <p>AI chat functionality coming soon...</p>
            </div>
          </div>
        </div>
      )}
    </>
  );
};