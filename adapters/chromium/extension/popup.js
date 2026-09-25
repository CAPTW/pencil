const [tab] = await chrome.tabs.query({active: true, currentWindow: true});
document.querySelector('#origin').textContent = tab?.url ? new URL(tab.url).origin : 'Unavailable';
for (const op of ['enable', 'disable', 'deny', 'allow', 'stop']) {
  document.querySelector('#' + op).onclick = async event => {
    if (!event.isTrusted) return;
    const response = await chrome.runtime.sendMessage({op, tabId: tab.id});
    document.querySelector('#status').textContent = response?.status || 'Unavailable';
  };
}
