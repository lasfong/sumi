import { NavLink } from 'react-router-dom';
import { Activity, LayoutDashboard, Database, LineChart, BookOpen, Settings, Cpu, Search, FlaskConical } from 'lucide-react';
import { useReplayStore } from '../../store/replayStore';
import './Sidebar.css';

interface SidebarProps {
  isCollapsed?: boolean;
  onToggleCollapse?: () => void;
}

export function Sidebar({ isCollapsed = false, onToggleCollapse }: SidebarProps) {
  const sessionId = useReplayStore((state) => state.sessionId);

  const getTargetPath = (basePath: string) => {
    if (!sessionId) return basePath;
    if (['/replay', '/journal', '/analytics'].includes(basePath)) {
      return `${basePath}?session=${sessionId}`;
    }
    return basePath;
  };

  const coreNavItems = [
    { path: '/', label: 'Dashboard', icon: LayoutDashboard },
    { path: '/replay', label: 'Trading Lab', icon: Activity, badge: 'Core' },
    { path: '/strategy-lab', label: 'Strategy Tester', icon: FlaskConical, badge: 'V3' },
    { path: '/import', label: 'Data Feeds', icon: Database },
    { path: '/backtest', label: 'Backtest Engine', icon: Cpu },
    { path: '/scanner', label: 'Signal Scanner', icon: Search },
    { path: '/analytics', label: 'Analytics', icon: LineChart },
    { path: '/journal', label: 'Journal', icon: BookOpen },
  ];

  return (
    <aside className={`sidebar glass-panel ${isCollapsed ? 'collapsed' : ''}`} data-testid="app-sidebar">
      <div className="sidebar-header">
        <div className="logo-glow" title="Sumi"></div>
        {!isCollapsed && (
          <>
            <h2>Sumi</h2>
            <span className="version-badge">v2.0</span>
          </>
        )}
        {onToggleCollapse && (
          <button
            type="button"
            className="collapse-toggle-btn"
            data-testid="sidebar-toggle-button"
            onClick={onToggleCollapse}
            title={isCollapsed ? 'Mở rộng menu' : 'Thu gọn menu (Zen mode)'}
            aria-label={isCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          >
            {isCollapsed ? '▶' : '◀'}
          </button>
        )}
      </div>

      <nav className="sidebar-nav">
        <div className="nav-group-title">{!isCollapsed ? 'TRADING WORKSPACE' : '•••'}</div>
        {coreNavItems.map((item) => {
          const targetPath = getTargetPath(item.path);
          return (
            <NavLink
              key={item.path}
              to={targetPath}
              className={({ isActive }) => `nav-item primary-nav-item ${isActive ? 'active' : ''}`}
              title={isCollapsed ? item.label : undefined}
            >
              <item.icon className="nav-icon" size={20} />
              {!isCollapsed && (
                <>
                  <span className="nav-label">{item.label}</span>
                  {item.badge && <span className="nav-core-badge">{item.badge}</span>}
                </>
              )}
            </NavLink>
          );
        })}
      </nav>

      <div className="sidebar-footer">
        <button className="nav-item config-btn" title={isCollapsed ? 'Cài đặt' : undefined}>
          <Settings className="nav-icon" size={20} />
          {!isCollapsed && <span className="nav-label">Settings</span>}
        </button>
      </div>
    </aside>
  );
}
