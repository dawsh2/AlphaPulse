/**
 * NotebookTemplatesList Component - Displays list of notebook templates
 * Extracted from ResearchPage notebook templates section
 */
import React from 'react';
import styles from '../../pages/ResearchPage.module.css';

interface NotebookCell {
  id: string;
  type: 'code' | 'markdown' | 'ai-chat';
  content: string;
  output?: string;
  isExecuting?: boolean;
  showAiAnalysis?: boolean;
  isAiChat?: boolean;
  aiMessages?: any[];
}

interface NotebookTemplate {
  id: string;
  title: string;
  description: string;
  cells: NotebookCell[];
}

interface NotebookTemplatesListProps {
  templates: NotebookTemplate[];
  onTemplateSelect: (template: NotebookTemplate) => void;
}

export const NotebookTemplatesList: React.FC<NotebookTemplatesListProps> = ({
  templates,
  onTemplateSelect
}) => {
  return (
    <div className={styles.templateList}>
      {templates.map(template => (
        <div 
          key={template.id} 
          className={styles.templateItem}
          onClick={() => onTemplateSelect(template)}
        >
          <div className={styles.templateName}>{template.title}</div>
          <div className={styles.templateDesc}>{template.description}</div>
        </div>
      ))}
    </div>
  );
};