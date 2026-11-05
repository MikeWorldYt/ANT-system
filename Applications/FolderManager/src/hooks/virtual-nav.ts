// Opening a virtual root entry, going back to a place in one, and leaving it for the real folder one of its entries stands for.
import { loadVirtual } from "../services/store/virtual";
import { leaveVirtual, notify, reveal, select, selectVirtual, state, type VirtualId } from "../services/store/state";
import { currentPlace, type Place } from "../services/store/place";
import { reportError } from "../components/organisms/bars/status-line";

export async function openVirtual(id: VirtualId): Promise<void> {
  selectVirtual(id);
  // Opening the entry reads Downloads afresh; later reloads keep what is loaded below it.
  if (id === "downloads") state.downloads = [];
  try {
    await loadVirtual();
  } catch (e) {
    reportError("error", e);
  }
  notify();
}

// The board is a toggle, so closing it returns to the place it was opened from.
let beforeBoard: Place | null = null;

export async function toggleBoard(): Promise<void> {
  if (state.virtual !== "board") {
    beforeBoard = currentPlace();
    return openVirtual("board");
  }
  const back = beforeBoard;
  beforeBoard = null;
  if (back === null || back.virtual === "board") return select(null);
  await goToPlace(back).catch((e: unknown) => reportError("error", e));
}

// The entry goes first, so `via` is set before the selection below it is revealed.
export async function goToPlace(place: Place): Promise<void> {
  if (place.virtual === null) return reveal(place.key?.split("/") ?? []);
  await openVirtual(place.virtual);
  if (place.via === null || state.virtual !== place.virtual) return;
  select(place.via.split("/"));
  if (place.key !== null && place.key !== place.via) await reveal(place.key.split("/"));
}

export async function goToReal(parts: string[]): Promise<void> {
  leaveVirtual();
  try {
    await reveal(parts);
  } catch (e) {
    reportError("error", e);
  }
}
