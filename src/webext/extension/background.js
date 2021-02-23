let port = browser.runtime.connectNative("vaxtify");

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

browser.tabs.onRemoved.addListener((tabId, removeInfo) => on_removed(tabId));
browser.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
    if (changeInfo.url !== undefined)
        on_updated(tabId, changeInfo.url)
});

browser.tabs.query({}).then(tabs => {
    for (let tab of tabs)
        on_updated(tab.id, tab.url);
});

port.onMessage.addListener(command => {
    if (command.kind === "Close")
        browser.tabs.remove(command.tab);
    else if (command.kind === "CreateEmpty")
        browser.tabs.create({});
});
