import allTools from './__fixtures__/valid-all-tools-document.json';
import horizontal from './__fixtures__/valid-horizontal-document.json';
import corpus from './__fixtures__/drawing-contract-corpus.json';
import { validateDrawingDocumentSemantics } from './drawingDomain';

type JsonRecord = Record<string, unknown>;
interface CorpusPatch {
  op: 'add' | 'replace' | 'remove'; path: string; value?: unknown;
  valueFrom?: string; repeat?: { value: string; count: number };
}
export interface DrawingContractCase {
  id: string; base: 'horizontal' | 'all-tools'; patches: CorpusPatch[];
  identity?: { sessionId: number; symbol: string };
  expectedStructural: boolean; expectedSemantic: boolean;
}

const decode = (part: string) => part.replace(/~1/g, '/').replace(/~0/g, '~');
const locate = (root: unknown, path: string): unknown => path.split('/').slice(1).map(decode)
  .reduce<unknown>((value, part) => Array.isArray(value) ? value[Number(part)] : (value as JsonRecord)[part], root);

export const materializeDrawingContractCase = (item: DrawingContractCase): unknown => {
  const document = structuredClone(item.base === 'horizontal' ? horizontal : allTools) as unknown;
  for (const patch of item.patches) {
    const parts = patch.path.split('/').slice(1).map(decode); const key = parts.pop()!;
    const parent = parts.reduce<unknown>((value, part) => Array.isArray(value) ? value[Number(part)] : (value as JsonRecord)[part], document);
    if (patch.op === 'remove') {
      if (Array.isArray(parent)) parent.splice(Number(key), 1); else delete (parent as JsonRecord)[key];
      continue;
    }
    const value = patch.valueFrom ? structuredClone(locate(document, patch.valueFrom))
      : patch.repeat ? patch.repeat.value.repeat(patch.repeat.count) : structuredClone(patch.value);
    if (Array.isArray(parent)) parent[Number(key)] = value; else (parent as JsonRecord)[key] = value;
  }
  return document;
};

export const DRAWING_CONTRACT_CORPUS = corpus as { schemaDraft: '2020-12'; semanticInvariants: string[]; cases: DrawingContractCase[] };
export const evaluateDrawingContractRuntimeCorpus = () => DRAWING_CONTRACT_CORPUS.cases.map(item => ({
  id: item.id, expectedSemantic: item.expectedSemantic,
  semantic: validateDrawingDocumentSemantics(materializeDrawingContractCase(item), item.identity),
}));
