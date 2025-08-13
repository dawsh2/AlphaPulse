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

interface NotebookTemplate {
  title: string;
  description: string;
  cells: Array<{
    type: 'code' | 'markdown';
    content: string;
  }>;
}

interface TemplateInfo {
  id: string;
  title: string;
  description: string;
}

class NotebookService {
  private baseUrl: string;

  constructor() {
    // Use FastAPI backend with notebook routes on port 8080
    this.baseUrl = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8080';
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

  /**
   * Get available notebook templates
   */
  async getTemplates(): Promise<TemplateInfo[]> {
    try {
      const response = await fetch(`${this.baseUrl}/api/notebook/templates`);
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const result = await response.json();
      return result.templates || [];
    } catch (error) {
      console.error('Error fetching templates:', error);
      return [];
    }
  }

  /**
   * Load a specific template
   */
  async loadTemplate(templateId: string): Promise<NotebookTemplate | null> {
    try {
      const response = await fetch(`${this.baseUrl}/api/notebook/templates/${templateId}`);
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      return await response.json();
    } catch (error) {
      console.error('Error loading template:', error);
      return null;
    }
  }
}

// Export singleton instance
export const notebookService = new NotebookService();

// Also export the class for testing
export { NotebookService };