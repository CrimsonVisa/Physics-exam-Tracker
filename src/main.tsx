import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './v2/App';
import { applyTheme, getStoredTheme } from './lib/uiPreferences';
import { applyAppearance, getStoredFontFamily, getStoredFontSize } from './lib/appearance';
import './styles.css';
import './v2/v2.css';
import './v2/design-system.css';
import './v2/workspaces.css';
import './v2/global-search.css';

applyTheme(getStoredTheme());
applyAppearance(getStoredFontFamily(), getStoredFontSize());

const rootElement = document.getElementById('root');

if (!rootElement) {
  throw new Error('Root element not found.');
}

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
