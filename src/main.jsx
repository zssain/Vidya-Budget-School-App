if (import.meta.env.MODE === 'mobile') {
  import('./main-mobile.jsx');
} else {
  import('./main-desktop.jsx');
}
