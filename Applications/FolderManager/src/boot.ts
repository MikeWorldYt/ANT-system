// Picks what this webview mounts: the dictionary editor or the transfer panel when the backend opened one, the main window otherwise.
declare global {
  interface Window {
    __DICTIONARY__?: string[];
    __TRANSFER__?: boolean;
  }
}

const folder = window.__DICTIONARY__;
if (folder) void import("./components/windows/dictionary-editor").then((m) => m.mountDictionaryEditor(folder));
else if (window.__TRANSFER__) void import("./components/organisms/transfer/transfer-window").then((m) => m.mountTransferWindow());
else void import("./main");

export {};
