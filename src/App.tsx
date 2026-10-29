import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

type AppInfo = {
  name: string;
  version: string;
};

type BackendState = 'checking' | 'connected' | 'unavailable';

export default function App() {
  const [backendState, setBackendState] = useState<BackendState>('checking');
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    let cancelled = false;

    invoke<AppInfo>('get_app_info')
      .then((info) => {
        if (!cancelled) {
          setAppInfo(info);
          setBackendState('connected');
        }
      })
      .catch(() => {
        if (!cancelled) {
          setBackendState('unavailable');
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <main className="app-shell">
      <section className="scaffold-card" aria-labelledby="app-title">
        <p className="eyebrow">Phase 1 scaffold</p>
        <h1 id="app-title">CSIR-NET Physics Tracker</h1>
        <p className="lede">
          A minimal Tauri 2 desktop shell for an offline-first CSIR-UGC NET Physical Sciences study
          tracker. Application features will be added in later phases.
        </p>

        <div className={`status status-${backendState}`} role="status" aria-live="polite">
          {backendState === 'checking' && 'Connecting to the Tauri backend…'}
          {backendState === 'connected' && appInfo && (
            <>
              Tauri backend connected · {appInfo.name} v{appInfo.version}
            </>
          )}
          {backendState === 'unavailable' &&
            'Tauri backend unavailable — run this window with npm run tauri:dev.'}
        </div>

        <dl className="stack-list">
          <div>
            <dt>Desktop shell</dt>
            <dd>Tauri 2</dd>
          </div>
          <div>
            <dt>Frontend</dt>
            <dd>React + TypeScript + Vite</dd>
          </div>
          <div>
            <dt>Backend</dt>
            <dd>Rust</dd>
          </div>
          <div>
            <dt>Data layer</dt>
            <dd>Deferred to Phase 2</dd>
          </div>
        </dl>

        <p className="footnote">
          No study data is stored yet. SQLite, backups, and application features arrive in later
          phases.
        </p>
      </section>
    </main>
  );
}
