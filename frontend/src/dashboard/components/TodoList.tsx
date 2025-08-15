import React, { useState, useEffect, useRef, KeyboardEvent } from 'react';
import './TodoList.css';

// Inline API for file persistence
const todosFileAPI = {
  async load(): Promise<any[]> {
    try {
      const response = await fetch('http://localhost:3001/api/todos');
      if (response.ok) {
        const data = await response.json();
        console.log(`Loaded ${data.length} todos from file`);
        return data;
      }
    } catch (error) {
      console.log('Todos server not running. Start with: npm run todos-server');
    }
    return [];
  },

  async save(todos: any[]): Promise<void> {
    try {
      const response = await fetch('http://localhost:3001/api/todos', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(todos)
      });
      
      if (response.ok) {
        const result = await response.json();
        console.log(`Saved ${result.count} todos to file`);
      }
    } catch (error) {
      console.log('Could not save to file. Todos server may not be running.');
    }
  }
};

interface Todo {
  id: string;
  text: string;
  completed: boolean;
  priority: 'low' | 'medium' | 'high';
  createdAt: Date;
  updatedAt: Date;
  parentId: string | null;
  collapsed: boolean;
  level: number;
}

export function TodoList() {
  // Initialize with empty array, will load async
  const [todos, setTodos] = useState<Todo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  
  const [newTodoText, setNewTodoText] = useState('');
  const [newTodoPriority, setNewTodoPriority] = useState<'low' | 'medium' | 'high'>('low');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingText, setEditingText] = useState('');
  const [filter, setFilter] = useState<'all' | 'active' | 'completed'>('all');
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const hasLoadedRef = useRef(false);

  // Load todos on mount from file AND localStorage
  useEffect(() => {
    const loadTodos = async () => {
      let loadedFromFile = false;
      
      // First try to load from file (survives cache clears)
      try {
        const fileTodos = await todosFileAPI.load();
        if (fileTodos && fileTodos.length > 0) {
          const todosFromFile = fileTodos.map((todo: any) => ({
            ...todo,
            createdAt: new Date(todo.createdAt),
            updatedAt: new Date(todo.updatedAt)
          }));
          setTodos(todosFromFile);
          loadedFromFile = true;
          console.log('Loaded todos from file:', todosFromFile.length);
        }
      } catch (error) {
        console.log('File API not available, will use localStorage');
      }
      
      // If no file data, try localStorage
      if (!loadedFromFile) {
        const savedTodos = localStorage.getItem('alphapulse-todos-org');
        if (savedTodos) {
          try {
            const parsed = JSON.parse(savedTodos);
            const todosFromStorage = parsed.map((todo: any) => ({
              ...todo,
              createdAt: new Date(todo.createdAt),
              updatedAt: new Date(todo.updatedAt)
            }));
            setTodos(todosFromStorage);
            console.log('Loaded todos from localStorage:', todosFromStorage.length);
          } catch (error) {
            console.error('Error parsing localStorage todos:', error);
          }
        }
      }
      
      hasLoadedRef.current = true;
      setIsLoading(false);
    };
    
    loadTodos();
  }, []);

  // Save todos whenever they change (after initial load)
  useEffect(() => {
    if (!hasLoadedRef.current) return;
    
    // Save to BOTH localStorage and file
    localStorage.setItem('alphapulse-todos-org', JSON.stringify(todos));
    
    // Also save to file (survives cache clears)
    todosFileAPI.save(todos);
  }, [todos]);

  // Export todos to JSON file (with optional auto-save to clipboard)
  const exportTodos = (autoSave = false) => {
    const dataStr = JSON.stringify(todos, null, 2);
    
    if (autoSave) {
      // Copy to clipboard for easy pasting
      navigator.clipboard.writeText(dataStr).then(() => {
        console.log('Todos copied to clipboard for backup');
      }).catch(err => {
        console.error('Failed to copy to clipboard:', err);
      });
    }
    
    const dataUri = 'data:application/json;charset=utf-8,'+ encodeURIComponent(dataStr);
    const exportFileDefaultName = `alphapulse-todos-${new Date().toISOString().split('T')[0]}.json`;
    
    const linkElement = document.createElement('a');
    linkElement.setAttribute('href', dataUri);
    linkElement.setAttribute('download', exportFileDefaultName);
    linkElement.click();
  };

  // Import todos from JSON file
  const importTodos = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;
    
    const reader = new FileReader();
    reader.onload = (e) => {
      try {
        const importedData = JSON.parse(e.target?.result as string);
        const importedTodos = importedData.map((todo: any) => ({
          ...todo,
          createdAt: new Date(todo.createdAt),
          updatedAt: new Date(todo.updatedAt)
        }));
        
        if (window.confirm(`Import ${importedTodos.length} todos? This will replace current todos.`)) {
          setTodos(importedTodos);
        }
      } catch (error) {
        alert('Error importing todos: Invalid file format');
      }
    };
    reader.readAsText(file);
    
    // Reset input
    event.target.value = '';
  };

  // Calculate the level of a todo based on its parent chain
  const calculateLevel = (todo: Todo, allTodos: Todo[]): number => {
    if (!todo.parentId) return 0;
    const parent = allTodos.find(t => t.id === todo.parentId);
    if (!parent) return 0;
    return calculateLevel(parent, allTodos) + 1;
  };

  // Get all descendant IDs of a todo
  const getDescendantIds = (todoId: string): string[] => {
    const descendants: string[] = [];
    const children = todos.filter(t => t.parentId === todoId);
    children.forEach(child => {
      descendants.push(child.id);
      descendants.push(...getDescendantIds(child.id));
    });
    return descendants;
  };

  // Check if a todo is visible (all parents are expanded)
  const isTodoVisible = (todo: Todo): boolean => {
    if (!todo.parentId) return true;
    const parent = todos.find(t => t.id === todo.parentId);
    if (!parent) return true;
    if (parent.collapsed) return false;
    return isTodoVisible(parent);
  };

  const addTodo = (parentId: string | null = null) => {
    if (!newTodoText.trim()) return;

    const newTodo: Todo = {
      id: Date.now().toString(),
      text: newTodoText.trim(),
      completed: false,
      priority: newTodoPriority,
      createdAt: new Date(),
      updatedAt: new Date(),
      parentId: parentId,
      collapsed: false,
      level: 0
    };

    // Calculate proper level
    const allTodos = [...todos, newTodo];
    newTodo.level = calculateLevel(newTodo, allTodos);

    // Insert after parent or selected item
    if (parentId) {
      const parentIndex = todos.findIndex(t => t.id === parentId);
      const siblings = todos.filter(t => t.parentId === parentId);
      const insertIndex = siblings.length > 0 
        ? todos.indexOf(siblings[siblings.length - 1]) + 1
        : parentIndex + 1;
      const newTodosList = [...todos];
      newTodosList.splice(insertIndex, 0, newTodo);
      setTodos(newTodosList);
    } else if (selectedId) {
      const selectedIndex = todos.findIndex(t => t.id === selectedId);
      const newTodosList = [...todos];
      newTodosList.splice(selectedIndex + 1, 0, newTodo);
      setTodos(newTodosList);
    } else {
      setTodos([newTodo, ...todos]);
    }

    setNewTodoText('');
    setNewTodoPriority('low');
  };

  const toggleCollapse = (id: string) => {
    setTodos(todos.map(todo => 
      todo.id === id 
        ? { ...todo, collapsed: !todo.collapsed }
        : todo
    ));
  };

  const toggleTodo = (id: string) => {
    const todo = todos.find(t => t.id === id);
    if (!todo) return;

    const descendantIds = getDescendantIds(id);
    const newCompleted = !todo.completed;

    setTodos(todos.map(t => {
      if (t.id === id || descendantIds.includes(t.id)) {
        return { ...t, completed: newCompleted, updatedAt: new Date() };
      }
      return t;
    }));
  };

  const deleteTodo = (id: string) => {
    const descendantIds = getDescendantIds(id);
    setTodos(todos.filter(t => t.id !== id && !descendantIds.includes(t.id)));
  };

  const startEditing = (todo: Todo) => {
    setEditingId(todo.id);
    setEditingText(todo.text);
  };

  const saveEdit = () => {
    if (!editingText.trim()) return;
    
    setTodos(todos.map(todo => 
      todo.id === editingId 
        ? { ...todo, text: editingText.trim(), updatedAt: new Date() }
        : todo
    ));
    setEditingId(null);
    setEditingText('');
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditingText('');
  };

  const changePriority = (id: string, priority: 'low' | 'medium' | 'high') => {
    setTodos(todos.map(todo => 
      todo.id === id 
        ? { ...todo, priority, updatedAt: new Date() }
        : todo
    ));
  };

  const promoteTodo = (id: string) => {
    const todo = todos.find(t => t.id === id);
    if (!todo) return;

    if (todo.parentId) {
      const parent = todos.find(t => t.id === todo.parentId);
      if (parent) {
        // Move to parent's parent
        setTodos(todos.map(t => 
          t.id === id 
            ? { ...t, parentId: parent.parentId, level: t.level - 1, updatedAt: new Date() }
            : t
        ));
      }
    }
  };

  const demoteTodo = (id: string) => {
    const todo = todos.find(t => t.id === id);
    if (!todo) return;

    const index = todos.indexOf(todo);
    if (index > 0) {
      const prevTodo = todos[index - 1];
      if (prevTodo.level === todo.level) {
        // Make it a child of previous sibling
        setTodos(todos.map(t => 
          t.id === id 
            ? { ...t, parentId: prevTodo.id, level: t.level + 1, updatedAt: new Date() }
            : t
        ));
      }
    }
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLDivElement>, todoId: string) => {
    const todo = todos.find(t => t.id === todoId);
    if (!todo) return;

    if (e.altKey && e.key === 'ArrowLeft') {
      e.preventDefault();
      promoteTodo(todoId);
    } else if (e.altKey && e.key === 'ArrowRight') {
      e.preventDefault();
      demoteTodo(todoId);
    } else if (e.key === 'Tab') {
      e.preventDefault();
      toggleCollapse(todoId);
    }
  };

  const filteredTodos = todos.filter(todo => {
    if (!isTodoVisible(todo)) return false;
    switch (filter) {
      case 'active': return !todo.completed;
      case 'completed': return todo.completed;
      default: return true;
    }
  });

  // Organize todos in tree structure for display
  const organizeTodos = (todosToOrganize: Todo[]): Todo[] => {
    const organized: Todo[] = [];
    const addWithChildren = (parentId: string | null, level: number = 0) => {
      const children = todosToOrganize
        .filter(t => t.parentId === parentId)
        .sort((a, b) => {
          if (a.completed !== b.completed) return a.completed ? 1 : -1;
          const priorityOrder = { high: 3, medium: 2, low: 1 };
          return priorityOrder[b.priority] - priorityOrder[a.priority];
        });
      
      children.forEach(child => {
        organized.push({ ...child, level });
        if (!child.collapsed) {
          addWithChildren(child.id, level + 1);
        }
      });
    };
    
    addWithChildren(null);
    return organized;
  };

  const organizedTodos = organizeTodos(filteredTodos);

  const stats = {
    total: todos.length,
    completed: todos.filter(t => t.completed).length,
    active: todos.filter(t => !t.completed).length,
    high: todos.filter(t => t.priority === 'high' && !t.completed).length
  };

  const getChildrenCount = (todoId: string): { total: number; completed: number } => {
    const children = todos.filter(t => t.parentId === todoId);
    let total = children.length;
    let completed = children.filter(c => c.completed).length;
    
    children.forEach(child => {
      const childStats = getChildrenCount(child.id);
      total += childStats.total;
      completed += childStats.completed;
    });
    
    return { total, completed };
  };

  if (isLoading) {
    return (
      <div className="todo-list org-mode">
        <div className="todo-header">
          <h2>Org Mode TODOs</h2>
        </div>
        <div className="loading-state">Loading todos...</div>
      </div>
    );
  }

  return (
    <div className="todo-list org-mode">
      <div className="todo-header">
        <h2>Org Mode TODOs</h2>
        <div className="todo-controls">
          <div className="todo-stats">
            <span className="stat">Total: {stats.total}</span>
            <span className="stat">Active: {stats.active}</span>
            <span className="stat">Completed: {stats.completed}</span>
            <span className="stat high-priority">High Priority: {stats.high}</span>
          </div>
          <div className="file-controls">
            <button onClick={() => exportTodos(false)} className="export-button" title="Export to JSON file - Save this file to survive cache clears!">
              📥 Export
            </button>
            <label className="import-button" title="Import from JSON file - Restore from your saved backup">
              📤 Import
              <input 
                type="file" 
                accept=".json" 
                onChange={importTodos}
                style={{ display: 'none' }}
              />
            </label>
          </div>
        </div>
      </div>

      <div className="todo-help">
        <span className="help-item">Tab: Fold/Unfold</span>
        <span className="help-item">Alt+←: Promote</span>
        <span className="help-item">Alt+→: Demote</span>
        <span className="help-item">Double-click: Edit</span>
      </div>

      <div className="todo-input-section">
        <div className="todo-input-row">
          <input
            ref={inputRef}
            type="text"
            value={newTodoText}
            onChange={(e) => setNewTodoText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                addTodo(null);
              } else if (e.key === 'Enter' && e.shiftKey && selectedId) {
                addTodo(selectedId);
              }
            }}
            placeholder={selectedId ? "Add TODO (Enter: sibling, Shift+Enter: child)" : "Add a new TODO item..."}
            className="todo-input"
          />
          <select
            value={newTodoPriority}
            onChange={(e) => setNewTodoPriority(e.target.value as 'low' | 'medium' | 'high')}
            className="priority-select"
          >
            <option value="low">Low</option>
            <option value="medium">Medium</option>
            <option value="high">High</option>
          </select>
          <button onClick={() => addTodo(null)} className="add-button">
            Add
          </button>
          {selectedId && (
            <button onClick={() => addTodo(selectedId)} className="add-button add-child">
              Add Child
            </button>
          )}
        </div>
      </div>

      <div className="todo-filters">
        <button 
          className={`filter-button ${filter === 'all' ? 'active' : ''}`}
          onClick={() => setFilter('all')}
        >
          All ({stats.total})
        </button>
        <button 
          className={`filter-button ${filter === 'active' ? 'active' : ''}`}
          onClick={() => setFilter('active')}
        >
          Active ({stats.active})
        </button>
        <button 
          className={`filter-button ${filter === 'completed' ? 'active' : ''}`}
          onClick={() => setFilter('completed')}
        >
          Completed ({stats.completed})
        </button>
      </div>

      <div className="todos-container">
        {organizedTodos.length === 0 ? (
          <div className="empty-state">
            {filter === 'all' 
              ? "No TODOs yet. Add one above to get started!" 
              : `No ${filter} TODOs.`
            }
          </div>
        ) : (
          organizedTodos.map(todo => {
            const childStats = getChildrenCount(todo.id);
            const hasChildren = childStats.total > 0;
            
            return (
              <div 
                key={todo.id} 
                className={`todo-item ${todo.completed ? 'completed' : ''} priority-${todo.priority} ${selectedId === todo.id ? 'selected' : ''}`}
                style={{ paddingLeft: `${todo.level * 24 + 8}px` }}
                onClick={() => setSelectedId(todo.id)}
                onKeyDown={(e) => handleKeyDown(e, todo.id)}
                tabIndex={0}
              >
                <div className="todo-main">
                  {hasChildren && (
                    <button 
                      className={`fold-button ${todo.collapsed ? 'collapsed' : ''}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        toggleCollapse(todo.id);
                      }}
                      title="Toggle fold (Tab)"
                    >
                      {todo.collapsed ? '▶' : '▼'}
                    </button>
                  )}
                  {!hasChildren && <span className="fold-spacer">•</span>}
                  
                  <input
                    type="checkbox"
                    checked={todo.completed}
                    onChange={() => toggleTodo(todo.id)}
                    className="todo-checkbox"
                    onClick={(e) => e.stopPropagation()}
                  />
                  
                  {editingId === todo.id ? (
                    <div className="editing-controls">
                      <input
                        type="text"
                        value={editingText}
                        onChange={(e) => setEditingText(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') saveEdit();
                          if (e.key === 'Escape') cancelEdit();
                        }}
                        onClick={(e) => e.stopPropagation()}
                        className="edit-input"
                        autoFocus
                      />
                      <button onClick={saveEdit} className="save-button">Save</button>
                      <button onClick={cancelEdit} className="cancel-button">Cancel</button>
                    </div>
                  ) : (
                    <>
                      <span 
                        className="todo-text" 
                        onDoubleClick={(e) => {
                          e.stopPropagation();
                          startEditing(todo);
                        }}
                      >
                        {todo.text}
                        {hasChildren && (
                          <span className="child-stats">
                            [{childStats.completed}/{childStats.total}]
                          </span>
                        )}
                      </span>
                      <div className="todo-actions">
                        <select
                          value={todo.priority}
                          onChange={(e) => changePriority(todo.id, e.target.value as 'low' | 'medium' | 'high')}
                          className={`priority-select priority-${todo.priority}`}
                          onClick={(e) => e.stopPropagation()}
                        >
                          <option value="low">Low</option>
                          <option value="medium">Med</option>
                          <option value="high">High</option>
                        </select>
                        <button 
                          onClick={(e) => {
                            e.stopPropagation();
                            promoteTodo(todo.id);
                          }}
                          className="promote-button"
                          title="Promote (Alt+←)"
                        >
                          ←
                        </button>
                        <button 
                          onClick={(e) => {
                            e.stopPropagation();
                            demoteTodo(todo.id);
                          }}
                          className="demote-button"
                          title="Demote (Alt+→)"
                        >
                          →
                        </button>
                        <button 
                          onClick={(e) => {
                            e.stopPropagation();
                            startEditing(todo);
                          }}
                          className="edit-button"
                          title="Edit (or double-click text)"
                        >
                          ✎
                        </button>
                        <button 
                          onClick={(e) => {
                            e.stopPropagation();
                            deleteTodo(todo.id);
                          }}
                          className="delete-button"
                          title="Delete (with children)"
                        >
                          ×
                        </button>
                      </div>
                    </>
                  )}
                </div>
              </div>
            );
          })
        )}
      </div>

      {todos.length > 0 && (
        <div className="todo-footer">
          <button 
            onClick={() => setTodos(todos.filter(t => !t.completed))}
            className="clear-completed-button"
            disabled={stats.completed === 0}
          >
            Clear Completed ({stats.completed})
          </button>
        </div>
      )}
    </div>
  );
}