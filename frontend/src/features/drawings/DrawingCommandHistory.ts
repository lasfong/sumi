import type { SumiDrawingDocumentV1 } from './drawingDomain';

export interface DrawingCommand { before: SumiDrawingDocumentV1; after: SumiDrawingDocumentV1; kind: 'create' | 'change' | 'delete' | 'clear' }

export class DrawingCommandHistory {
  private undoStack: DrawingCommand[] = [];
  private redoStack: DrawingCommand[] = [];
  commit(command: DrawingCommand): void { this.undoStack.push(structuredClone(command)); this.redoStack = []; }
  peekUndo(): DrawingCommand | null { const item = this.undoStack.at(-1); return item ? structuredClone(item) : null; }
  acceptUndo(): void { const item = this.undoStack.pop(); if (item) this.redoStack.push(item); }
  peekRedo(): DrawingCommand | null { const item = this.redoStack.at(-1); return item ? structuredClone(item) : null; }
  acceptRedo(): void { const item = this.redoStack.pop(); if (item) this.undoStack.push(item); }
  get canUndo(): boolean { return this.undoStack.length > 0; }
  get canRedo(): boolean { return this.redoStack.length > 0; }
  snapshot(): { undo: DrawingCommand[]; redo: DrawingCommand[] } { return { undo: structuredClone(this.undoStack), redo: structuredClone(this.redoStack) }; }
  clear(): void { this.undoStack = []; this.redoStack = []; }
}
