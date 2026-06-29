export const toDateKey = (value?: string | null): string | null => {
  if (!value) return null;
  if (value.includes('T')) return value.split('T')[0];
  if (value.includes(' ')) return value.split(' ')[0];
  return value;
};

export const unixSecondsToDateKey = (seconds: number): string => (
  new Date(seconds * 1000).toISOString().split('T')[0]
);

export const sortDateKeys = (a: string, b: string): number => a.localeCompare(b);
