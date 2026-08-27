if (import.meta.env.VITE_TAURI_E2E === '1') {
  void import('@wdio/tauri-plugin');
}
