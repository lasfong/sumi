import { useState } from 'react';
import { NavLink, useLocation } from 'react-router-dom';
import {
  Activity,
  BookOpen,
  LineChart,
  FlaskConical,
  Search,
  Layers,
  Sliders,
  Database,
  Settings,
} from 'lucide-react';
import { useReplayStore } from '../../store/replayStore';
import { SettingsModal } from '../settings/SettingsModal';
import './Sidebar.css';

interface SidebarProps {
  isCollapsed?: boolean;
  onToggleCollapse?: () => void;
}

interface NavItemDef {
  path: string;
  label: string;
  icon: React.ComponentType<{ size?: number; className?: string }>;
  badge?: string;
  testId: string;
  preserveSession?: boolean;
}

interface NavGroupDef {
  id: string;
  title: string;
  testId: string;
  items: NavItemDef[];
}

export function Sidebar({ isCollapsed = false, onToggleCollapse }: SidebarProps) {
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const location = useLocation();
  const sessionId = useReplayStore((state) => state.sessionId);

  const getTargetPath = (basePath: string, preserveSession?: boolean) => {
    if (preserveSession && sessionId) {
      return `${basePath}?session=${sessionId}`;
    }
    return basePath;
  };

  const isItemActive = (targetPath: string) => {
    const [baseRoute, query] = targetPath.split('?');
    if (query) {
      return location.pathname === baseRoute && location.search.includes(query);
    }
    if (location.pathname === baseRoute) {
      if (baseRoute === '/strategy-lab' && location.search.includes('tab=')) {
        return false;
      }
      return true;
    }
    return false;
  };

  const navGroups: NavGroupDef[] = [
    {
      id: 'manual-practice',
      title: 'LUYỆN TẬP THỦ CÔNG (MANUAL PRACTICE)',
      testId: 'nav-group-manual',
      items: [
        {
          path: '/replay',
          label: 'Trading Lab',
          icon: Activity,
          badge: 'Core',
          testId: 'nav-item-replay',
          preserveSession: true,
        },
        {
          path: '/journal',
          label: 'Nhật Ký Giao Dịch',
          icon: BookOpen,
          testId: 'nav-item-journal',
          preserveSession: true,
        },
        {
          path: '/analytics',
          label: 'Phân Tích Hiệu Suất',
          icon: LineChart,
          testId: 'nav-item-analytics',
          preserveSession: true,
        },
      ],
    },
    {
      id: 'auto-lab',
      title: 'KIỂM ĐỊNH TỰ ĐỘNG & NGHIÊN CỨU (AUTO TEST & LAB)',
      testId: 'nav-group-auto',
      items: [
        {
          path: '/strategy-lab',
          label: 'Strategy Tester',
          icon: FlaskConical,
          badge: 'V3',
          testId: 'nav-item-strategy-lab',
        },
        {
          path: '/scanner',
          label: 'Bộ Quét Tín Hiệu',
          icon: Search,
          testId: 'nav-item-scanner',
        },
        {
          path: '/strategy-lab?tab=catalog',
          label: 'Thư Viện Tín Hiệu',
          icon: Layers,
          testId: 'nav-item-signal-catalog',
        },
        {
          path: '/strategy-lab?tab=builder',
          label: 'Soạn Quy Tắc',
          icon: Sliders,
          testId: 'nav-item-rule-builder',
        },
      ],
    },
    {
      id: 'system-utils',
      title: 'TIỆN ÍCH HỆ THỐNG',
      testId: 'nav-group-system',
      items: [
        {
          path: '/import',
          label: 'Nạp Dữ Liệu',
          icon: Database,
          testId: 'nav-item-import',
        },
      ],
    },
  ];

  return (
    <>
      <aside className={`sidebar glass-panel ${isCollapsed ? 'collapsed' : ''}`} data-testid="app-sidebar">
        <div className="sidebar-header">
          <NavLink to="/" className="sidebar-brand" title="Sumi - Dashboard">
            <div className="logo-glow" title="Sumi"></div>
            {!isCollapsed && (
              <>
                <h2>Sumi</h2>
                <span className="version-badge" data-testid="version-badge">v3.0.0</span>
              </>
            )}
          </NavLink>
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
          {navGroups.map((group) => (
            <div key={group.id} className="nav-group-section">
              <div className="nav-group-title" data-testid={group.testId}>
                {!isCollapsed ? group.title : '•••'}
              </div>
              {group.items.map((item) => {
                const targetPath = getTargetPath(item.path, item.preserveSession);
                const active = isItemActive(targetPath);
                return (
                  <NavLink
                    key={item.path}
                    to={targetPath}
                    data-testid={item.testId}
                    className={`nav-item primary-nav-item ${active ? 'active' : ''}`}
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
            </div>
          ))}
        </nav>

        <div className="sidebar-footer">
          <button
            type="button"
            className="nav-item config-btn"
            data-testid="sidebar-settings-button"
            onClick={() => setIsSettingsOpen(true)}
            title={isCollapsed ? 'Cài đặt' : undefined}
          >
            <Settings className="nav-icon" size={20} />
            {!isCollapsed && <span className="nav-label">Cài Đặt</span>}
          </button>
        </div>
      </aside>

      <SettingsModal isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} />
    </>
  );
}
