import { NavLink } from 'react-router-dom';
import { Activity, LayoutDashboard, Database, LineChart, BookOpen, Settings, Cpu, Search, FlaskConical } from 'lucide-react';
import { useReplayStore } from '../../store/replayStore';
import './Sidebar.css';

export function Sidebar() {
  const sessionId = useReplayStore((state) => state.sessionId);

  const getTargetPath = (basePath: string) => {
    if (!sessionId) return basePath;
    if (['/replay', '/journal', '/analytics'].includes(basePath)) {
      return `${basePath}?session=${sessionId}`;
    }
    return basePath;
  };

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
        <span className="version-badge">v2.0.0-rc2</span>
      </div>
      
      <nav className="sidebar-nav">
        {navItems.map((item) => {
          const targetPath = getTargetPath(item.path);
          return (
            <NavLink
              key={item.path}
              to={targetPath}
              className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}
            >
              <item.icon className="nav-icon" size={20} />
              <span className="nav-label">{item.label}</span>
            </NavLink>
          );
        })}
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
