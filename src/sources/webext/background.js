let port = browser.runtime.connectNative("vaxtify");

function on_created(tabId) {
    port.postMessage({
        "kind": "Created",
        "timestamp": new Date().toISOString(),
        "tab": tabId
    });
}
function on_removed(tabId) {
    port.postMessage({
        "kind": "Removed",
        "timestamp": new Date().toISOString(),
        "tab": tabId
    });
}
function on_updated(tabId, url) {
    if (url !== undefined) {
        port.postMessage({
            "kind": "Updated",
            "timestamp": new Date().toISOString(),
            "tab": tabId,
            "url": url
        });
    }
}
function on_activated(tabId) {
    port.postMessage({
        "kind": "Activated",
        "timestamp": new Date().toISOString(),
        "tab": tabId
    });
}

browser.tabs.onCreated.addListener(tab => on_created(tab.id));
browser.tabs.onRemoved.addListener((tabId, removeInfo) => on_removed(tabId));
browser.tabs.onUpdated.addListener((tabId, changeInfo, tab) => on_updated(tabId, changeInfo.url));
browser.tabs.onActivated.addListener(activeInfo => on_activated(activeInfo.tabId));

browser.tabs.query({}).then(tabs => {
    for (let tab of tabs) {
        on_created(tab.id);
        on_updated(tab.id, tab.url);
        if (tab.active)
            on_activated(tab.id);
    }
});
