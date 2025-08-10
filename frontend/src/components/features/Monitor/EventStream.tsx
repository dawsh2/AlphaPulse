/**
 * Event stream component for live trading events
 */

import React, { useEffect, useRef } from 'react';
import { formatTimeAgo } from '../../../utils/format';
import styles from './Monitor.module.css';

interface Event {
  id: string;
  time: number;
  type: 'buy' | 'sell' | 'signal' | 'error' | 'info';
  description: string;
  metadata?: Record<string, any>;
}

interface EventStreamProps {
  events: Event[];
  maxEvents?: number;
  autoScroll?: boolean;
}

export const EventStream: React.FC<EventStreamProps> = ({
  events,
  maxEvents = 100,
  autoScroll = true,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new events arrive
  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [events, autoScroll]);

  // Limit displayed events
  const displayedEvents = events.slice(-maxEvents);

  const getEventIcon = (type: Event['type']) => {
    switch (type) {
      case 'buy': return '🟢';
      case 'sell': return '🔴';
      case 'signal': return '📊';
      case 'error': return '⚠️';
      case 'info': return 'ℹ️';
      default: return '•';
    }
  };

  const getEventClass = (type: Event['type']) => {
    switch (type) {
      case 'buy': return styles.eventBuy;
      case 'sell': return styles.eventSell;
      case 'signal': return styles.eventSignal;
      case 'error': return styles.eventError;
      default: return styles.eventInfo;
    }
  };

  return (
    <div className={styles.eventStream} ref={containerRef}>
      {displayedEvents.map((event) => (
        <div key={event.id} className={`${styles.eventItem} ${getEventClass(event.type)}`}>
          <div className={styles.eventHeader}>
            <span className={styles.eventIcon}>{getEventIcon(event.type)}</span>
            <span className={styles.eventTime}>{formatTimeAgo(event.time)}</span>
          </div>
          <div className={styles.eventDescription}>{event.description}</div>
          {event.metadata && (
            <div className={styles.eventMetadata}>
              {Object.entries(event.metadata).map(([key, value]) => (
                <span key={key} className={styles.metadataItem}>
                  {key}: {JSON.stringify(value)}
                </span>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
};