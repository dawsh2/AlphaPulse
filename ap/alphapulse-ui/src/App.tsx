import React from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { Layout } from './components/Layout/Layout';
import { NewsPage } from './pages/NewsPage';
import { DevelopPage } from './pages/DevelopPage';
import ResearchPage from './pages/ResearchPage';
import MonitorPage from './pages/MonitorPage';
import './styles/theme.css';

function App() {
  return (
    <Router>
      <Layout>
        <Routes>
          <Route path="/" element={<NewsPage />} />
          <Route path="/develop" element={<DevelopPage />} />
          <Route path="/research" element={<ResearchPage />} />
          <Route path="/monitor" element={<MonitorPage />} />
          <Route path="/login" element={<div style={{ padding: '2rem' }}>Login page coming soon...</div>} />
        </Routes>
      </Layout>
    </Router>
  );
}

export default App;
