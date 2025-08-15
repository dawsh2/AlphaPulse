const express = require('express');
const fs = require('fs');
const path = require('path');
const cors = require('cors');

const app = express();
const PORT = 3001;
const TODOS_FILE = path.join(__dirname, '..', 'todos-data.json');

app.use(cors());
app.use(express.json());

// GET /api/todos - Load todos from file
app.get('/api/todos', (req, res) => {
  try {
    if (fs.existsSync(TODOS_FILE)) {
      const data = fs.readFileSync(TODOS_FILE, 'utf8');
      res.json(JSON.parse(data));
    } else {
      res.json([]);
    }
  } catch (error) {
    console.error('Error reading todos:', error);
    res.json([]);
  }
});

// POST /api/todos - Save todos to file
app.post('/api/todos', (req, res) => {
  try {
    const todos = req.body;
    fs.writeFileSync(TODOS_FILE, JSON.stringify(todos, null, 2));
    res.json({ success: true, count: todos.length });
  } catch (error) {
    console.error('Error saving todos:', error);
    res.status(500).json({ success: false, error: error.message });
  }
});

app.listen(PORT, () => {
  console.log(`Todos persistence server running on http://localhost:${PORT}`);
  console.log(`Todos are saved to: ${TODOS_FILE}`);
});