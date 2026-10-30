// Tauri event subscriptions kept out of api.ts, which owns only command calls.
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { DragDropEvent } from "@tauri-apps/api/webview";
import type { ReviewIndexEvent, StorageChangedEvent } from "./types";

export const onReviewIndex = (fn: (e: ReviewIndexEvent) => void) =>
  listen<ReviewIndexEvent>("fm://review-index", (ev) => fn(ev.payload));

export const onWindowFocus = (fn: () => void) =>
  getCurrentWindow().onFocusChanged(({ payload }) => {
    if (payload) fn();
  });

export const onFileDrag = (fn: (e: DragDropEvent) => void) =>
  getCurrentWebview().onDragDropEvent(({ payload }) => fn(payload));

export const onStorageChanged =(fn: (e: StorageChangedEvent) => void) =>
  listen<StorageChangedEvent>("storage-changed", (ev) => fn(ev.payload));
