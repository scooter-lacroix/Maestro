import { useEffect } from 'react';
import { Dashboard } from './components/Dashboard';
import './App.css';

function App() {
  useEffect(() => {
    // Initialize theme from localStorage
    const savedTheme = localStorage.getItem('dashboard-theme') || 'dark';
    document.body.className = `theme-${savedTheme}`;

    // Load data display mode from localStorage
    const savedDataDisplay = localStorage.getItem('dashboard-data-display');
    if (savedDataDisplay) {
      document.body.setAttribute('data-display', savedDataDisplay);
    }
  }, []);

  return <Dashboard />;
}

export default App;
