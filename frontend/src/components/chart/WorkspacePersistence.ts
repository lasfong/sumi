import { SumiDrawingAdapter } from './SumiDrawingAdapter';
import type { DrawingLine } from './workspaceTypes';

export interface WorkspaceState {
  drawings: DrawingLine[];
  indicators: Array<{
    name: string;
    pane: 'main' | 'oscillator';
    params: Record<string, string | number | boolean>;
    color?: string;
  }>;
}

export class WorkspacePersistence {
  static serialize(state: WorkspaceState): string {
    return JSON.stringify({
      version: 1,
      drawings: JSON.parse(SumiDrawingAdapter.serialize(state.drawings)),
      indicators: state.indicators,
    });
  }

  static deserialize(value?: string | null): WorkspaceState {
    if (!value) return { drawings: [], indicators: [] };
    try {
      const parsed = JSON.parse(value) as { drawings?: unknown; indicators?: unknown };
      return {
        drawings: SumiDrawingAdapter.deserialize(JSON.stringify(parsed.drawings ?? [])),
        indicators: Array.isArray(parsed.indicators)
          ? parsed.indicators.filter(WorkspacePersistence.isIndicator)
          : [],
      };
    } catch {
      return { drawings: [], indicators: [] };
    }
  }

  static load(sessionId?: number | null): WorkspaceState {
    if (!sessionId || typeof window === 'undefined') return { drawings: [], indicators: [] };
    return WorkspacePersistence.deserialize(window.localStorage.getItem(`sumi:workspace:${sessionId}`));
  }

  static save(sessionId: number, state: WorkspaceState): void {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(`sumi:workspace:${sessionId}`, WorkspacePersistence.serialize(state));
  }

  private static isIndicator(value: unknown): value is WorkspaceState['indicators'][number] {
    if (!value || typeof value !== 'object') return false;
    const indicator = value as Record<string, unknown>;
    return typeof indicator.name === 'string'
      && (indicator.pane === 'main' || indicator.pane === 'oscillator')
      && typeof indicator.params === 'object' && indicator.params !== null;
  }
}
