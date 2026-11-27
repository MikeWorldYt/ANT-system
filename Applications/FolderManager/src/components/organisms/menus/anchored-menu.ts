// Small dropdown menu that opens under a button and closes on pick, Escape or an outside click.
import { el } from "../../dom";
import type { GuiIcon } from "../../atoms/gui-icons";
import { focusInitial, menuIcon, menuItem, openPopup, place, placeUnder, popupTrigger, rove, separator } from "../../molecules/menu";

export interface MenuChoice {
  label: string;
  // Present for radio-style menus, where exactly one choice is checked.
  checked?: boolean;
  // Shown in place of the checked dot; `flip` turns it upside down, as Descending reuses the up arrow.
  icon?: GuiIcon;
  flip?: boolean;
  // A picture drawn by the caller, such as a folder's own icon.
  lead?: () => HTMLElement;
  // Draws a separator above this choice.
  divider?: boolean;
  danger?: boolean;
  run(): void;
}

let menuCount = 0;
// About where `.ctx-iconed` draws its icon's centre, nudged so the labels meet the crumb text.
const ICON_CENTER = 15;

// `over` opens the menu from the anchor's corner, covering it, as row kebab menus do.
// `under` opens it below the whole bar with the icon column centred under the anchor, so the labels line up past it.
export function attachMenu(anchor: HTMLElement, label: string, choices: () => MenuChoice[], over = false, under = false): void {
  let closeMenu: ((restore?: boolean) => void) | null = null;
  const menuId = `anchored-menu-${++menuCount}`;
  const close = (restore = true) => closeMenu?.(restore);

  function open(): void {
    const list = choices();
    const items = list.map((c) => {
      const item = menuItem(c.label, c.checked !== undefined ? "menuitemradio" : "menuitem", true, () => {
        close();
        c.run();
      }, ["ctx-item", ...(c.icon || c.lead ? ["ctx-iconed"] : []), ...(c.danger ? ["danger"] : [])].join(" "), c.checked);
      if (c.icon) item.prepend(menuIcon(c.icon, c.flip));
      else if (c.lead) {
        const lead = c.lead();
        lead.classList.add("ctx-icon", "ctx-lead");
        item.prepend(lead);
      }
      return item;
    });
    const menu = el("div", { id: menuId, class: "ctx-menu", role: "menu", "aria-label": label }, el("div", { class: "ctx-group", role: "group" }, ...items.flatMap((b, i) => (list[i]?.divider ? [separator(), b] : [b]))));
    menu.addEventListener("keydown", (ev) => {
      if (ev.key === "Tab") {
        close();
        return;
      }
      if (ev.key === "Escape") {
        close();
        ev.preventDefault();
      } else rove(items, ev);
    });
    menu.addEventListener("focusout", (ev) => {
      if (!menu.contains(ev.relatedTarget as Node | null)) close(false);
    });
    // Keeps a press on the menu padding from blurring the focused item, which would close the menu.
    menu.addEventListener("mousedown", (ev) => ev.preventDefault());
    const r = anchor.getBoundingClientRect();
    closeMenu = openPopup(anchor, menu, () => (over ? place(menu, r.left, r.top) : under ? place(menu, r.left + r.width / 2 - ICON_CENTER, (anchor.closest(".breadcrumb")?.getBoundingClientRect().bottom ?? r.bottom) + 3) : placeUnder(menu, r)), () => (closeMenu = null));
    focusInitial(items);
  }

  anchor.setAttribute("aria-haspopup", "menu");
  anchor.setAttribute("aria-expanded", "false");
  anchor.setAttribute("aria-controls", menuId);
  popupTrigger(anchor, ["ArrowDown"], () => closeMenu, open);
}
