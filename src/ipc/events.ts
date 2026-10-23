// Tauri event subscriptions kept out of api.ts, which owns only command calls.
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ReviewIndexEvent } from "./types";

export const onReviewIndex = (fn: (e: ReviewIndexEvent) => void) =>
  listen<ReviewIndexEvent>("fm://review-index", (ev) => fn(ev.payload));

export const onWindowFocus = (fn: () => void) =>
  getCurrentWindow().onFocusChanged(({ payload }) => {
    if (payload) fn();
  });
