// Closing the WebView can destroy the quit IPC reply. Callers MUST separately
// await the owned process exit and assert exitCode === 0; this is not exit proof.
export async function requestAppQuit(page) {
  try {
    await page.evaluate(() => window.__TAURI_INTERNALS__.invoke('quit_app'));
    return {reply: 'received'};
  } catch (error) {
    if (!/Target page, context or browser has been closed/.test(String(error))) throw error;
    return {reply: 'transport_closed_before_reply', error: String(error)};
  }
}
