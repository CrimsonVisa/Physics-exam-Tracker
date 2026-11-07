(() => {
  const themeKey = 'csir-net-tracker.ui.theme.v1';
  const fontFamilyKey = 'csir-net-tracker.ui.font-family.v1';
  const fontSizeKey = 'csir-net-tracker.ui.font-size.v1';
  let theme = 'dark';
  let fontFamily = 'inter';
  let fontSize = 'default';
  const themeColors = { dark: '#0e1319', light: '#f3f5f8' };

  try {
    const storedTheme = window.localStorage.getItem(themeKey);
    if (storedTheme === 'light' || storedTheme === 'dark') {
      theme = storedTheme;
    }
    const storedFamily = window.localStorage.getItem(fontFamilyKey);
    if (storedFamily === 'manrope' || storedFamily === 'ibm-plex' || storedFamily === 'jakarta') {
      fontFamily = storedFamily;
    }
    const storedSize = window.localStorage.getItem(fontSizeKey);
    if (storedSize === 'compact' || storedSize === 'comfortable') {
      fontSize = storedSize;
    }
  } catch (e) {
    // Storage can be unavailable; dark, Inter and default size remain safe.
  }

  document.documentElement.dataset.theme = theme;
  document.documentElement.dataset.fontFamily = fontFamily;
  document.documentElement.dataset.fontSize = fontSize;
  document.documentElement.style.colorScheme = theme;

  const themeColor = document.querySelector('meta[name="theme-color"]');
  if (themeColor) {
    themeColor.setAttribute('content', themeColors[theme]);
  }
})();
