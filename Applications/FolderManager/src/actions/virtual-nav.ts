// Opening a virtual root entry, and leaving it for the real folder one of its entries stands for.
import { describeError } from "../ipc/api";
import { loadVirtual } from "../store/virtual";
import { notify, reveal, select, state, type VirtualId } from "../store/state";
import { setStatus } from "../ui/status";

export async function openVirtual(id: VirtualId): Promise<void> {
  select(null);
  state.virtual = id;
  notify();
  try {
    await loadVirtual();
  } catch (e) {
    setStatus("error", describeError(e));
  }
  notify();
}

export async function goToReal(parts: string[]): Promise<void> {
  state.virtual = null;
  state.virtualKeys = [];
  try {
    await reveal(parts);
  } catch (e) {
    setStatus("error", describeError(e));
  }
}
