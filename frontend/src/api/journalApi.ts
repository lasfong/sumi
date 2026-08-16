import { apiClient } from './client';
import type { JournalEntry, JournalEntryCreate } from '../types';

/** Create a new journal entry for a session */
export const createJournalEntry = async (
  sessionId: number,
  entry: JournalEntryCreate
): Promise<JournalEntry> => {
  const response = await apiClient.post(
    `/replay/sessions/${sessionId}/journal`,
    entry
  );
  return response.data;
};

/** Get all journal entries for a session */
export const getJournalEntries = async (
  sessionId: number
): Promise<JournalEntry[]> => {
  const response = await apiClient.get(
    `/replay/sessions/${sessionId}/journal`
  );
  return response.data;
};

/** Export journal data in json or csv */
export const exportJournal = async (
  sessionId: number,
  format: 'json' | 'csv' = 'json'
): Promise<unknown> => {
  const response = await apiClient.get(`/replay/sessions/${sessionId}/journal/export`, {
    params: { format },
    responseType: format === 'csv' ? 'text' : 'json',
  });
  return response.data;
};
