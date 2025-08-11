/**
 * Monitor page type definitions
 */

export interface EventData {
  time: string;
  type: 'signal' | 'order' | 'fill' | 'error' | 'info';
  description: string;
  details?: any;
}

// Re-export for convenience
export type { EventData as MonitorEventData };