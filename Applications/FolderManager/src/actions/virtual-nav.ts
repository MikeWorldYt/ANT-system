// Opening a virtual root entry, going back to a place in one, and leaving it for the real folder one of its entries stands for.
import { describeError } from "../ipc/api";
import { loadVirtual } from "../store/virtual";
import { leaveVirtual, notify, reveal, select, selectVirtual, state, type VirtualId } from "../store/state";
import { currentPlace, type Place } from "../store/place";
import { setStatus } from "../ui/status";

export async function openVirtual(id: VirtualId): Promise<void> {
  selectVirtual(id);
  try {
    await loadVirtual();
  } catch (e) {
    setStatus("error", describeError(e));
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
  await goToPlace(back).catch((e: unknown) => setStatus("error", describeError(e)));
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
    setStatus("error", describeError(e));
  }
}
