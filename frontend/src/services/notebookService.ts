/**
 * Notebook Service - Handles communication with Jupyter backend
 */

interface ExecuteResult {
  output: string | null;
  error: string | null;
  images?: string[] | null;
}

interface KernelStatus {
  status: 'running' | 'stopped' | 'error';
  kernel?: string;
  error?: string;
}

class NotebookService {
  private baseUrl: string;

  constructor() {
    // Use Flask backend with notebook routes on port 5002
    this.baseUrl = import.meta.env.VITE_API_BASE_URL || 'http://localhost:5002';
  }

  /**
   * Execute Python code in the Jupyter kernel
   */
  async executeCode(code: string): Promise<ExecuteResult> {
    try {
      const response = await fetch(`${this.baseUrl}/api/notebook/execute`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ code }),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const result = await response.json();
      return result;
    } catch (error) {
      console.error('Error executing code:', error);
      return {
        output: null,
        error: error instanceof Error ? error.message : 'Unknown error occurred',
      };
    }
  }

  /**
   * Check kernel status
   */
  async getStatus(): Promise<KernelStatus> {
    try {
      const response = await fetch(`${this.baseUrl}/api/notebook/status`);
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      return await response.json();
    } catch (error) {
      return {
        status: 'error',
        error: error instanceof Error ? error.message : 'Unknown error occurred',
      };
    }
  }

  /**
   * Restart the kernel
   */
  async restartKernel(): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/api/notebook/restart`, {
        method: 'POST',
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const result = await response.json();
      return result.status === 'restarted';
    } catch (error) {
      console.error('Error restarting kernel:', error);
      return false;
    }
  }
}

// Export singleton instance
export const notebookService = new NotebookService();

// Also export the class for testing
export { NotebookService };