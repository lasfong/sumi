import { NavLink } from 'react-router-dom';
import { Activity, LayoutDashboard, Database, LineChart, BookOpen, Settings, Cpu, Search, FlaskConical } from 'lucide-react';
import './Sidebar.css';

export function Sidebar() {
  const navItems = [
    { path: '/', label: 'Dashboard', icon: LayoutDashboard },
    { path: '/import', label: 'Data Feeds', icon: Database },
    { path: '/replay', label: 'Trading Lab', icon: Activity },
    { path: '/backtest', label: 'Backtest Engine', icon: Cpu },
    { path: '/strategy-lab', label: 'Strategy Lab', icon: FlaskConical },
    { path: '/scanner', label: 'Signal Scanner', icon: Search },
    { path: '/analytics', label: 'Analytics', icon: LineChart },
    { path: '/journal', label: 'Journal', icon: BookOpen },
  ];

  return (
    <aside className="sidebar glass-panel">
      <div className="sidebar-header">
        <div className="logo-glow"></div>
        <h2>Sumi</h2>
      </div>
      
      <nav className="sidebar-nav">
        {navItems.map((item) => (
          <NavLink 
            key={item.path} 
            to={item.path} 
            className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}
          >
            <item.icon className="nav-icon" size={20} />
            <span className="nav-label">{item.label}</span>
          </NavLink>
        ))}
      </nav>

      <div className="sidebar-footer">
        <button className="nav-item config-btn">
          <Settings className="nav-icon" size={20} />
          <span className="nav-label">Settings</span>
        </button>
      </div>
    </aside>
  );
}
