// Picks what this webview mounts: the dictionary editor, a properties window or the transfer panel when the backend opened one, the main window otherwise.
import type { PropertiesTarget } from "./components/windows/properties-window";

declare global {
  interface Window {
    __DICTIONARY__?: string[];
    __PROPERTIES__?: PropertiesTarget;
    __TRANSFER__?: boolean;
  }
}

const folder = window.__DICTIONARY__;
const properties = window.__PROPERTIES__;
if (folder) void import("./components/windows/dictionary-editor").then((m) => m.mountDictionaryEditor(folder));
else if (properties) void import("./components/windows/properties-window").then((m) => m.mountPropertiesWindow(properties));
else if (window.__TRANSFER__) void import("./components/organisms/transfer/transfer-window").then((m) => m.mountTransferWindow());
else void import("./main");

export {};
