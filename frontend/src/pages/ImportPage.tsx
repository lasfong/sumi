import React, { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { importCafefData } from '../api/importApi';
import toast from 'react-hot-toast';

export const ImportPage: React.FC = () => {
  const [file, setFile] = useState<File | null>(null);
  const [result, setResult] = useState<any>(null);
  const [error, setError] = useState<string | null>(null);

  const mutation = useMutation({
    mutationFn: (fileObj: File) => importCafefData(fileObj),
    onSuccess: (data) => {
      setResult(data);
      setError(null);
      toast.success('Data imported successfully!');
    },
    onError: (err: any) => {
      const errMsg = err?.response?.data?.detail || 'Import failed. Please check the file format.';
      setError(errMsg);
      setResult(null);
      toast.error(errMsg);
    },
  });

  const handleImport = () => {
    if (file) {
      mutation.mutate(file);
    }
  };

  return (
    <div style={{ padding: '2rem', maxWidth: '700px', margin: '0 auto' }}>
      <h2 style={{ marginBottom: '0.5rem' }}>📂 Import CafeF Data</h2>
      <p style={{ color: 'var(--text-muted)', marginBottom: '2rem', fontSize: '14px' }}>
        Upload CSV, TXT, or ZIP files from CafeF to populate your trading data.
      </p>

      <div className="panel" style={{ marginBottom: '1.5rem' }}>
        <div style={{ display: 'flex', gap: '1rem', alignItems: 'center' }}>
          <label
            style={{
              flex: 1, padding: '1.5rem', border: '2px dashed var(--border-color)',
              borderRadius: '8px', textAlign: 'center', cursor: 'pointer',
              color: file ? 'var(--color-buy)' : 'var(--text-muted)',
              transition: 'border-color 0.2s',
            }}
          >
            <input
              type="file"
              accept=".csv,.txt,.zip"
              onChange={(e) => setFile(e.target.files?.[0] || null)}
              style={{ display: 'none' }}
            />
            {file ? `📄 ${file.name} (${(file.size / 1024).toFixed(1)} KB)` : '📎 Click to select file (.csv, .txt, .zip)'}
          </label>
          <button
            onClick={handleImport}
            disabled={!file || mutation.isPending}
            style={{
              background: 'var(--color-primary)', color: 'white',
              padding: '12px 24px', fontSize: '14px', fontWeight: 700,
              whiteSpace: 'nowrap',
            }}
          >
            {mutation.isPending ? '⏳ Importing...' : '📥 Import'}
          </button>
        </div>
      </div>

      {error && (
        <div className="panel" style={{ borderColor: 'var(--color-sell)', marginBottom: '1rem' }}>
          <p style={{ color: 'var(--color-sell)', margin: 0 }}>❌ {error}</p>
        </div>
      )}

      {result && (
        <div className="panel">
          <h3 style={{ marginTop: 0, color: 'var(--color-buy)' }}>✅ Import Successful</h3>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '1rem', marginBottom: '1rem' }}>
            <div className="card">
              <h4>Imported Rows</h4>
              <p>{result.imported_rows}</p>
            </div>
            <div className="card">
              <h4>Symbols Found</h4>
              <p>{result.symbols_count}</p>
            </div>
            <div className="card">
              <h4>Skipped Rows</h4>
              <p style={{ color: result.skipped_rows > 0 ? 'var(--color-sell)' : 'var(--color-buy)' }}>
                {result.skipped_rows}
              </p>
            </div>
            <div className="card">
              <h4>Date Range</h4>
              <p style={{ fontSize: '14px' }}>{result.start_date || '—'}<br/>to {result.end_date || '—'}</p>
            </div>
          </div>
          {result.warnings?.length > 0 && (
            <div style={{ marginTop: '1rem' }}>
              <h4 style={{ color: 'var(--color-close)' }}>⚠️ Warnings ({result.warnings.length})</h4>
              <div style={{ maxHeight: '200px', overflowY: 'auto', fontSize: '13px' }}>
                {result.warnings.slice(0, 20).map((w: any, i: number) => (
                  <div key={i} style={{ padding: '4px 0', borderBottom: '1px solid var(--border-color)' }}>
                    <span style={{ color: 'var(--text-muted)' }}>Row {w.row_index}:</span> {w.message}
                  </div>
                ))}
                {result.warnings.length > 20 && (
                  <p style={{ color: 'var(--text-muted)', fontStyle: 'italic' }}>...and {result.warnings.length - 20} more</p>
                )}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
