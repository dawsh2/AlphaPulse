import React, { useEffect, useState } from 'react';
import Editor from '@monaco-editor/react';
import * as monaco from 'monaco-editor';
import styles from './CodeEditor.module.css';

interface CodeEditorProps {
  value: string;
  onChange: (value: string) => void;
  language?: string;
}

// Map common language extensions to Monaco language IDs
const getMonacoLanguage = (language?: string): string => {
  switch (language) {
    case 'python':
      return 'python';
    case 'yaml':
    case 'yml':
      return 'yaml';
    case 'json':
      return 'json';
    case 'markdown':
      return 'markdown';
    case 'javascript':
    case 'js':
      return 'javascript';
    case 'typescript':
    case 'ts':
      return 'typescript';
    case 'html':
      return 'html';
    case 'css':
      return 'css';
    default:
      return 'text';
  }
};

export const CodeEditor: React.FC<CodeEditorProps> = ({ value, onChange, language = 'python' }) => {
  // Initialize with correct theme detection
  const [theme, setTheme] = useState(() => {
    const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
                   (!document.documentElement.getAttribute('data-theme') && 
                    window.matchMedia('(prefers-color-scheme: dark)').matches);
    return isDark ? 'vs-dark' : 'cream-light';
  });
  
  useEffect(() => {
    // Check if monaco is available before defining theme
    if (typeof monaco !== 'undefined' && monaco.editor) {
      // Define the cream theme once
      monaco.editor.defineTheme('cream-light', {
        base: 'vs',
        inherit: true,
        rules: [],
        colors: {
          'editor.background': '#faf7f0', // Cream/eggshell color
          'editor.foreground': '#33332d',
          'editor.lineHighlightBackground': '#f5f2ea',
          'editor.selectionBackground': '#e5e0d5',
          'editorCursor.foreground': '#33332d',
          'editorLineNumber.foreground': '#8b8680',
          'editorLineNumber.activeForeground': '#33332d'
        }
      });
    }
    
    // Detect current theme
    const updateTheme = () => {
      const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
                     (!document.documentElement.getAttribute('data-theme') && 
                      window.matchMedia('(prefers-color-scheme: dark)').matches);
      
      setTheme(isDark ? 'vs-dark' : 'cream-light');
    };
    
    updateTheme();
    
    // Listen for theme changes
    const observer = new MutationObserver(updateTheme);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme']
    });
    
    return () => observer.disconnect();
  }, []);

  const handleEditorChange = (newValue: string | undefined) => {
    onChange(newValue || '');
  };

  const handleEditorDidMount = (editor: monaco.editor.IStandaloneCodeEditor, monaco: any) => {
    // Define the cream theme with more explicit colors
    monaco.editor.defineTheme('cream-light', {
      base: 'vs',
      inherit: true,
      rules: [
        { token: '', foreground: '33332d', background: 'faf7f0' }
      ],
      colors: {
        'editor.background': '#faf7f0',
        'editor.foreground': '#33332d',
        'editorLineNumber.foreground': '#8b8680',
        'editorLineNumber.activeForeground': '#33332d',
        'editor.selectionBackground': '#e5e0d5',
        'editor.lineHighlightBackground': '#f5f2ea',
        'editorCursor.foreground': '#33332d',
        'editorWidget.background': '#f5f2ea',
        'editorSuggestWidget.background': '#f5f2ea',
        'editorHoverWidget.background': '#f5f2ea'
      }
    });
    
    // Force apply the theme
    monaco.editor.setTheme(theme);
  };

  return (
    <div className={styles.editorContainer}>
      <Editor
        height="100%"
        language={getMonacoLanguage(language)}
        value={value}
        onChange={handleEditorChange}
        theme={theme}
        onMount={handleEditorDidMount}
        options={{
          fontSize: 13,
          lineHeight: 1.5,
          fontFamily: "'IBM Plex Mono', 'SF Mono', Monaco, Consolas, 'Courier New', monospace",
          minimap: { enabled: false },
          scrollBeyondLastLine: false,
          automaticLayout: true,
          wordWrap: 'on',
          lineNumbers: 'on',
          folding: true,
          selectOnLineNumbers: true,
          matchBrackets: 'always',
          autoIndent: 'advanced',
          formatOnPaste: true,
          formatOnType: true,
          tabSize: 4,
          insertSpaces: true,
          renderWhitespace: 'boundary',
          smoothScrolling: true,
          cursorBlinking: 'smooth',
          cursorSmoothCaretAnimation: 'on'
        }}
      />
    </div>
  );
};

export default CodeEditor;