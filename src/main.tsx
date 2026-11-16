import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './v2/App';
import { applyTheme, getStoredTheme } from './lib/uiPreferences';
import { applyAppearance, getStoredFontFamily, getStoredFontSize } from './lib/appearance';
import 'katex/dist/katex.min.css';
import './styles.css';
import './v2/v2.css';
import './v2/design-system.css';
import './v2/math.css';
import './v2/workspaces.css';
import './v2/global-search.css';
import './v2/flashcards.css';
import './v2/calculator.css';
import './v2/formulas.css';
import './v2/terms.css';
import './v2/reference.css';
import './v2/notes/notes.css';
import './v2/charts/charts.css';

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
