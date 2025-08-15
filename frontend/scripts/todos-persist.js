#!/usr/bin/env node

/**
 * Simple todos persistence script
 * Run this to save/load todos to a JSON file that survives cache clearing
 * 
 * Usage:
 *   node scripts/todos-persist.js save '<json-data>'
 *   node scripts/todos-persist.js load
 */

const fs = require('fs');
const path = require('path');

const TODOS_FILE = path.join(__dirname, '..', 'todos-data.json');

const command = process.argv[2];
const data = process.argv[3];

switch (command) {
  case 'save':
    if (!data) {
      console.error('Usage: node todos-persist.js save \'<json-data>\'');
      process.exit(1);
    }
    try {
      const todos = JSON.parse(data);
      fs.writeFileSync(TODOS_FILE, JSON.stringify(todos, null, 2));
      console.log(`Saved ${todos.length} todos to ${TODOS_FILE}`);
    } catch (error) {
      console.error('Error saving todos:', error.message);
      process.exit(1);
    }
    break;

  case 'load':
    try {
      if (fs.existsSync(TODOS_FILE)) {
        const content = fs.readFileSync(TODOS_FILE, 'utf8');
        console.log(content);
      } else {
        console.log('[]');
      }
    } catch (error) {
      console.error('Error loading todos:', error.message);
      process.exit(1);
    }
    break;

  default:
    console.log('Todos Persistence Script');
    console.log('Usage:');
    console.log('  node scripts/todos-persist.js save \'<json-data>\'');
    console.log('  node scripts/todos-persist.js load');
    console.log('');
    console.log('The todos are saved to todos-data.json in the frontend directory.');
    console.log('This file persists even when browser cache is cleared.');
}