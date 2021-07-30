let port = browser.runtime.connectNative("vaxtify");

// Will be resent by the proxy for every restart of vaxtify, because the extension can't know when
// that happens.
port.postMessage({
    "kind": "Handshake",
    "version": browser.runtime.getManifest().version
});

function on_removed(tabId) {
    port.postMessage({
        "kind": "Removed",
        "tab": tabId
    });
}
function on_updated(tabId, url) {
    port.postMessage({
        "kind": "Updated",
        "tab": tabId,
        "url": url
    });
}

async function refresh() {
    let tabs = await browser.tabs.query({});
    for (let tab of tabs)
        on_updated(tab.id, tab.url);
}

browser.tabs.onRemoved.addListener((tabId, removeInfo) => on_removed(tabId));
browser.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
    if (changeInfo.url !== undefined)
        on_updated(tabId, changeInfo.url)
});

port.onMessage.addListener(command => {
    if (command.kind === "Close")
        browser.tabs.remove(command.tab);
    else if (command.kind === "CreateEmpty")
        browser.tabs.create({});
    else if (command.kind === "Refresh")
        refresh();
    else
        console.warn('unexpected message from vaxtify webext proxy', command);
});

refresh();
