const [tab] = await chrome.tabs.query({active: true, currentWindow: true});
document.querySelector('#origin').textContent = tab?.url ? new URL(tab.url).origin : 'Unavailable';
for (const op of ['enable', 'disable', 'deny', 'allow', 'stop', 'deep-consent', 'deep-revoke', 'cleanup-recover']) {
  document.querySelector('#' + op).onclick = async event => {
    if (!event.isTrusted) return;
    const response = await chrome.runtime.sendMessage({op,tabId:tab.id,provider:document.querySelector('#provider').value,consent:document.querySelector('#consent').checked});
    document.querySelector('#status').textContent = response?.status || 'Unavailable';
  };
}
