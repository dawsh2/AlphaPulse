/**
 * Terminal Service
 * Handles command execution and terminal session management
 */

interface ExecuteResult {
  output: string;
  exit_code?: number;
  session_id: string;
  cwd?: string;
  error?: string;
}

class TerminalService {
  private sessionId: string | null = null;
  private isProduction = window.location.hostname !== 'localhost' && window.location.hostname !== '127.0.0.1';
  
  /**
   * Execute a command in the terminal
   */
  async execute(command: string): Promise<ExecuteResult> {
    // SECURITY: Disable terminal execution in production
    if (this.isProduction) {
      return {
        output: 'Terminal execution is disabled in production for security reasons.',
        session_id: 'disabled',
        error: 'Terminal disabled in production'
      };
    }
    
    try {
      // Generate session ID if not exists
      if (!this.sessionId) {
        this.sessionId = `session-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
      }
      
      const response = await fetch('http://localhost:5001/api/terminal/execute', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          command,
          session_id: this.sessionId
        })
      });
      
      const result = await response.json();
      
      // Update session ID if returned
      if (result.session_id) {
        this.sessionId = result.session_id;
      }
      
      return result;
    } catch (error) {
      return {
        output: `Error: Failed to execute command: ${error}`,
        session_id: this.sessionId || 'error',
        error: String(error)
      };
    }
  }
  
  /**
   * Get autocomplete suggestions for a partial command
   */
  async getAutocomplete(partial: string): Promise<string[]> {
    try {
      const response = await fetch('http://localhost:5001/api/terminal/autocomplete', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          partial,
          session_id: this.sessionId
        })
      });
      
      if (response.ok) {
        const result = await response.json();
        return result.suggestions || [];
      }
      
      return [];
    } catch (error) {
      console.error('Autocomplete error:', error);
      return [];
    }
  }
  
  /**
   * Close the current terminal session
   */
  async closeSession(): Promise<void> {
    if (!this.sessionId) return;
    
    try {
      await fetch(`http://localhost:5001/api/terminal/session/${this.sessionId}`, {
        method: 'DELETE'
      });
    } catch (error) {
      console.error('Error closing session:', error);
    } finally {
      this.sessionId = null;
    }
  }
  
  /**
   * Get current session ID
   */
  getSessionId(): string | null {
    return this.sessionId;
  }
  
  /**
   * Create a new session (resets current session)
   */
  newSession(): void {
    this.sessionId = null;
  }
}

// Export singleton instance
export const terminalService = new TerminalService();

// Export type for use in components
export type { ExecuteResult };