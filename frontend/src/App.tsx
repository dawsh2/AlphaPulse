// React 19 JSX transform handles imports automatically
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { useEffect } from 'react';
import { Layout } from './components/Layout/Layout';
import { NewsPage } from './pages/NewsPage';
import { DevelopPage } from './pages/DevelopPage';
import ResearchPage from './pages/ResearchPage';
import MonitorPage from './pages/MonitorPage';
import './styles/theme.css';

function App() {
  useEffect(() => {
    // Suppress Monaco clipboard errors globally
    const originalConsoleError = console.error;
    console.error = (...args) => {
      const errorString = args.join(' ');
      // Suppress clipboard permission errors from Monaco Editor
      if (errorString.includes('NotAllowedError') && 
          (errorString.includes('clipboard') || 
           errorString.includes('clipboardService'))) {
        return;
      }
      originalConsoleError.apply(console, args);
    };

    // Also suppress via window.onerror
    const originalWindowError = window.onerror;
    window.onerror = (message, source, lineno, colno, error) => {
      if (error?.name === 'NotAllowedError' && 
          (String(message).includes('clipboard') || 
           String(source).includes('clipboard'))) {
        return true; // Prevent error from being logged
      }
      if (originalWindowError) {
        return originalWindowError(message, source, lineno, colno, error);
      }
      return false;
    };

    // Cleanup
    return () => {
      console.error = originalConsoleError;
      window.onerror = originalWindowError;
    };
  }, []);
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
