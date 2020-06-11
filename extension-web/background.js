let port = browser.runtime.connectNative("distraction_oni");

browser.tabs.onCreated.addListener(tab => port.postMessage({
    "kind": "Created",
    "tab": tab.id
}));
browser.tabs.onRemoved.addListener((tabId, removeInfo) => port.postMessage({
    "kind": "Removed",
    "tab": tabId
}));
browser.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
    if (changeInfo.url !== undefined) {
        port.postMessage({
            "kind": "Updated",
            "tab": tabId,
            "url": changeInfo.url
        });
    }
});
browser.tabs.onActivated.addListener(activeInfo => port.postMessage({
    "kind": "Activated",
    "tab": activeInfo.tabId
}));
