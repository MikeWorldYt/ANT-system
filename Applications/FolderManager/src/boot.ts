// Picks what this webview mounts: the dictionary editor when the backend opened it for one, the main window otherwise.
declare global {
  interface Window {
    __DICTIONARY__?: string[];
  }
}

const folder = window.__DICTIONARY__;
if (folder) void import("./components/windows/dictionary-editor").then((m) => m.mountDictionaryEditor(folder));
else void import("./main");

export {};
