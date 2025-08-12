/**
 * Code Editor wrapper component for development environment
 */

import React, { useRef, useEffect, useCallback } from 'react';
import * as monaco from 'monaco-editor';
import styles from './Develop.module.css';

interface CodeEditorProps {
  value: string;
  language?: string;
  theme?: 'vs-dark' | 'vs-light' | 'hc-black';
  onChange?: (value: string) => void;
  onSave?: (value: string) => void;
  readOnly?: boolean;
  height?: string;
  options?: monaco.editor.IStandaloneEditorConstructionOptions;
}

export const CodeEditor: React.FC<CodeEditorProps> = ({
  value,
  language = 'python',
  theme = 'vs-dark',
  onChange,
  onSave,
  readOnly = false,
  height = '100%',
  options = {},
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);

  // Initialize editor
  useEffect(() => {
    if (!containerRef.current) return;

    // Suppress clipboard errors globally
    const originalError = window.onerror;
    window.onerror = (message, source, lineno, colno, error) => {
      if (error?.name === 'NotAllowedError' && typeof message === 'string' && message.includes('clipboard')) {
        return true; // Prevent error from being logged
      }
      if (originalError) {
        return originalError(message, source, lineno, colno, error);
      }
      return false;
    };

    // Configure Python language
    monaco.languages.setMonarchTokensProvider('python', {
      defaultToken: '',
      tokenPostfix: '.python',
      keywords: [
        'False', 'None', 'True', 'and', 'as', 'assert', 'async', 'await',
        'break', 'class', 'continue', 'def', 'del', 'elif', 'else', 'except',
        'finally', 'for', 'from', 'global', 'if', 'import', 'in', 'is',
        'lambda', 'nonlocal', 'not', 'or', 'pass', 'raise', 'return',
        'try', 'while', 'with', 'yield'
      ],
      builtins: [
        'abs', 'all', 'any', 'ascii', 'bin', 'bool', 'bytearray', 'bytes',
        'callable', 'chr', 'classmethod', 'compile', 'complex', 'delattr',
        'dict', 'dir', 'divmod', 'enumerate', 'eval', 'exec', 'filter',
        'float', 'format', 'frozenset', 'getattr', 'globals', 'hasattr',
        'hash', 'help', 'hex', 'id', 'input', 'int', 'isinstance',
        'issubclass', 'iter', 'len', 'list', 'locals', 'map', 'max',
        'memoryview', 'min', 'next', 'object', 'oct', 'open', 'ord',
        'pow', 'print', 'property', 'range', 'repr', 'reversed', 'round',
        'set', 'setattr', 'slice', 'sorted', 'staticmethod', 'str',
        'sum', 'super', 'tuple', 'type', 'vars', 'zip'
      ],
      tokenizer: {
        root: [
          [/[a-z_]\w*/i, {
            cases: {
              '@keywords': 'keyword',
              '@builtins': 'type.identifier',
              '@default': 'identifier'
            }
          }],
          [/[A-Z]\w*/, 'type.identifier'],
          [/#.*$/, 'comment'],
          [/"""/, 'string', '@doubleDocString'],
          [/'''/, 'string', '@singleDocString'],
          [/"/, 'string', '@doubleString'],
          [/'/, 'string', '@singleString'],
          [/\d+\.\d+/, 'number.float'],
          [/\d+/, 'number'],
          [/[+\-*/%=<>!&|^~:]/, 'operator'],
          [/[()[\]{}]/, '@brackets'],
          [/[,.]/, 'delimiter'],
        ],
        doubleDocString: [
          [/[^"]+/, 'string'],
          [/"""/, 'string', '@pop'],
          [/"/, 'string']
        ],
        singleDocString: [
          [/[^']+/, 'string'],
          [/'''/, 'string', '@pop'],
          [/'/, 'string']
        ],
        doubleString: [
          [/[^"\\]+/, 'string'],
          [/\\./, 'string.escape'],
          [/"/, 'string', '@pop']
        ],
        singleString: [
          [/[^'\\]+/, 'string'],
          [/\\./, 'string.escape'],
          [/'/, 'string', '@pop']
        ]
      }
    });

    // Register completion provider for Python
    monaco.languages.registerCompletionItemProvider('python', {
      provideCompletionItems: (model, position) => {
        const word = model.getWordUntilPosition(position);
        const range = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn
        };

        const suggestions = [
          // AlphaPulse specific
          { label: 'Strategy', kind: monaco.languages.CompletionItemKind.Class, insertText: 'Strategy', range },
          { label: 'Indicator', kind: monaco.languages.CompletionItemKind.Class, insertText: 'Indicator', range },
          { label: 'Signal', kind: monaco.languages.CompletionItemKind.Class, insertText: 'Signal', range },
          { label: 'backtest', kind: monaco.languages.CompletionItemKind.Function, insertText: 'backtest(${1:strategy}, ${2:data})', range },
          { label: 'optimize', kind: monaco.languages.CompletionItemKind.Function, insertText: 'optimize(${1:params})', range },
          
          // Common imports
          { label: 'import pandas as pd', kind: monaco.languages.CompletionItemKind.Snippet, insertText: 'import pandas as pd', range },
          { label: 'import numpy as np', kind: monaco.languages.CompletionItemKind.Snippet, insertText: 'import numpy as np', range },
          { label: 'from alphapulse import', kind: monaco.languages.CompletionItemKind.Snippet, insertText: 'from alphapulse import ${1:Strategy}', range },
        ];

        return { suggestions };
      }
    });

    const editor = monaco.editor.create(containerRef.current, {
      value,
      language,
      theme,
      readOnly,
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: 14,
      lineNumbers: 'on',
      roundedSelection: false,
      scrollBeyondLastLine: false,
      wordWrap: 'on',
      contextmenu: false, // Disable context menu to prevent clipboard errors
      ...options,
    });

    editorRef.current = editor;

    // Handle changes
    const changeDisposable = editor.onDidChangeModelContent(() => {
      const currentValue = editor.getValue();
      onChange?.(currentValue);
    });

    // Handle save shortcut
    const saveDisposable = editor.addCommand(
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS,
      () => {
        onSave?.(editor.getValue());
      }
    );

    return () => {
      changeDisposable.dispose();
      editor.dispose();
    };
  }, [language, theme, readOnly, options]);

  // Update value when prop changes
  useEffect(() => {
    if (editorRef.current && value !== editorRef.current.getValue()) {
      editorRef.current.setValue(value);
    }
  }, [value]);

  // Update theme
  useEffect(() => {
    monaco.editor.setTheme(theme);
  }, [theme]);

  const handleFormat = useCallback(() => {
    editorRef.current?.getAction('editor.action.formatDocument')?.run();
  }, []);

  return (
    <div className={styles.codeEditorWrapper}>
      <div className={styles.editorToolbar}>
        <div className={styles.editorTitle}>
          {language === 'python' ? '🐍' : '📄'} Editor
        </div>
        <div className={styles.editorActions}>
          <button
            className={styles.editorButton}
            onClick={handleFormat}
            title="Format Code (Alt+Shift+F)"
          >
            Format
          </button>
          {onSave && (
            <button
              className={styles.editorButton}
              onClick={() => onSave(editorRef.current?.getValue() || '')}
              title="Save (Cmd+S)"
            >
              Save
            </button>
          )}
        </div>
      </div>
      <div
        ref={containerRef}
        className={styles.monacoContainer}
        style={{ height }}
      />
    </div>
  );
};