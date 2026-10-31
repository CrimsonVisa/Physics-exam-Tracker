(() => {
  const themeKey = 'csir-net-tracker.ui.theme.v1';
  let theme = 'dark';

  try {
    const stored = window.localStorage.getItem(themeKey);
    if (stored === 'light' || stored === 'dark') {
      theme = stored;
    }
  } catch {
    // Storage can be unavailable; dark remains the safe default.
  }

  document.documentElement.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme;
})();
