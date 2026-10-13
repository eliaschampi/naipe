// Shared primitives. Every screen uses the same controls, piles, and sheets.
import { cardEl } from "./cards";
import { redSuit, SUITS, SUIT_NAMES } from "./types";

/** Enter and Space fire the same click a pointer would. */
export function keyClick(node: HTMLElement): void {
  node.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      node.click();
    }
  });
}

export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  cls = "",
  text = "",
): HTMLElementTagNameMap[K] {
  const d = document.createElement(tag);
  if (cls) d.className = cls;
  if (text) d.textContent = text;
  return d;
}

export function button(label: string, cls = "", onClick?: () => void): HTMLButtonElement {
  const b = el("button", `btn ${cls}`.trim(), label);
  b.type = "button";
  if (onClick) b.onclick = onClick;
  return b;
}

export function topbar(opts: {
  onHome: () => void;
  onRules: () => void;
  onNew: () => void;
  canNew: boolean;
  extra?: HTMLElement[];
  soundBtn: HTMLButtonElement;
}): HTMLElement {
  const bar = el("div", "topbar");
  const brand = el("div", "brand");
  brand.innerHTML = "NAI<em>P</em>E";
  brand.onclick = opts.onHome;
  brand.tabIndex = 0;
  brand.setAttribute("role", "button");
  keyClick(brand);
  const spacer = el("div", "spacer");
  bar.append(brand, spacer);
  (opts.extra ?? []).forEach((e) => bar.append(e));
  bar.append(opts.soundBtn);
  bar.append(button("Rules", "ghost small", opts.onRules));
  const anew = button("New", "ghost small", opts.onNew);
  anew.dataset.role = "new";
  anew.disabled = !opts.canNew;
  bar.append(anew);
  bar.append(button("Games", "ghost small", opts.onHome));
  return bar;
}

export function soundButton(getLabel: () => string, onToggle: () => void): HTMLButtonElement {
  const b = el("button", "icon-btn", getLabel());
  b.type = "button";
  b.title = "Sound";
  b.setAttribute("aria-label", "Sound");
  b.onclick = () => {
    onToggle();
    b.textContent = getLabel();
  };
  return b;
}

export function playerSelect(value: number, onChange: (n: number) => void): HTMLSelectElement {
  const s = el("select", "seg");
  s.setAttribute("aria-label", "Players");
  for (const n of [2, 3, 4]) {
    const o = el("option", "", `${n} players`);
    o.value = String(n);
    if (n === value) o.selected = true;
    s.append(o);
  }
  s.onchange = () => onChange(Number(s.value));
  return s;
}

export interface OverlayOpts {
  /** False for a choice the rules require, such as calling a suit. */
  dismiss?: boolean;
  mark?: string;
}

export function overlay(
  title: string,
  body: HTMLElement,
  onClose: () => void,
  opts: OverlayOpts = {},
): HTMLElement {
  const dismiss = opts.dismiss !== false;
  const ov = el("div", "overlay");
  if (opts.mark) ov.classList.add(opts.mark);
  const sheet = el("div", "sheet");
  sheet.setAttribute("role", "dialog");
  sheet.setAttribute("aria-modal", "true");
  sheet.setAttribute("aria-label", title);
  const h = el("h2", "", title);
  sheet.append(h, body);
  const close = (): void => {
    ov.remove();
    onClose();
  };
  if (dismiss) {
    const row = el("div", "row");
    row.append(button("Close", "small", close));
    sheet.append(row);
    ov.addEventListener("click", (e) => {
      if (e.target === ov) close();
    });
    const esc = (e: KeyboardEvent): void => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      close();
    };
    document.addEventListener("keydown", esc);
    const mo = new MutationObserver(() => {
      if (!ov.isConnected) {
        document.removeEventListener("keydown", esc);
        mo.disconnect();
      }
    });
    mo.observe(document.body, { childList: true, subtree: true });
  }
  ov.append(sheet);
  return ov;
}

export function finale(
  title: string,
  message: string,
  againLabel: string,
  onAgain: () => void,
): HTMLElement {
  const body = el("div");
  body.append(el("p", "", message));
  const row = el("div", "row");
  const sheet = overlay(title, body, () => {}, { mark: "finale" });
  row.append(button(againLabel, "primary", () => {
    sheet.remove();
    onAgain();
  }));
  body.append(row);
  return sheet;
}

export function suitSheet(onPick: (suit: number) => void): HTMLElement {
  const body = el("div");
  body.append(el("p", "", "Your 8 is wild. The suit you call stays live until someone changes it."));
  const grid = el("div", "suit-grid");
  SUITS.forEach((glyph, s) => {
    const b = el("button");
    b.type = "button";
    if (redSuit(s)) b.className = "red";
    b.innerHTML = `${glyph}<small>${SUIT_NAMES[s]}</small>`;
    b.onclick = () => onPick(s);
    grid.append(b);
  });
  body.append(grid);
  const sheet = overlay("Call the suit", body, () => {}, { dismiss: false, mark: "suit-overlay" });
  return sheet;
}

export function rulesBody(items: string[]): HTMLElement {
  const wrap = el("div");
  const ul = el("ul");
  items.forEach((t) => ul.append(el("li", "", t)));
  wrap.append(ul);
  return wrap;
}

export function logEl(lines: string[]): HTMLElement {
  const d = el("div", "log");
  d.setAttribute("aria-label", "Recent plays");
  lines.slice(-4).forEach((l) => d.append(el("div", "", l)));
  return d;
}

export function errorLine(): HTMLElement {
  const d = el("div", "err");
  d.setAttribute("role", "status");
  return d;
}

export interface ScoreChip {
  name: string;
  stat: string;
  title?: string;
  active?: boolean;
  you?: boolean;
}

/** One rail for every multiplayer table: who is playing, and the number that matters. */
export function scoreRail(chips: ScoreChip[]): HTMLElement {
  const wrap = el("div", "score-rail");
  wrap.setAttribute("aria-label", "Players");
  chips.forEach((c) => {
    const chip = el("div", `score${c.active ? " active" : ""}${c.you ? " you" : ""}`);
    if (c.title) chip.title = c.title;
    chip.append(el("span", "who", c.name), el("span", "stat", c.stat));
    wrap.append(chip);
  });
  return wrap;
}

/** Status, never clickable: the suit that is currently live. */
export function suitBadge(suit: number): HTMLElement {
  const b = el("span", "suit-badge");
  const red = redSuit(suit);
  const g = el("span", `g${red ? " red" : ""}`, SUITS[suit]);
  b.append(g, el("span", "", SUIT_NAMES[suit]));
  return b;
}

export function pileWrap(o: {
  card: HTMLElement;
  /** A caption string, or a ready-made cap such as a suit mark. Omit for none. */
  label?: string | HTMLElement;
  enabled?: boolean;
  title?: string;
  onClick?: () => void;
}): HTMLElement {
  const on = Boolean(o.enabled && o.onClick);
  const wrap = el("div", `pile-wrap${on ? " actionable" : o.enabled === false ? " idle" : ""}`);
  const pile = el("div", "pile");
  if (on && o.onClick) {
    o.card.onclick = o.onClick;
    if (o.title) {
      o.card.title = o.title;
      pile.title = o.title;
    }
  }
  pile.append(o.card);
  wrap.append(pile);
  if (o.label instanceof HTMLElement) wrap.append(o.label);
  else if (o.label) wrap.append(el("div", "pile-cap", o.label));
  return wrap;
}

/** The one clickable draw pile. Idle when drawing is illegal. */
export function drawPile(o: {
  count: number;
  enabled: boolean;
  label?: string;
  title?: string;
  onClick: () => void;
}): HTMLElement {
  return pileWrap({
    card: cardEl(0, { faceDown: true, interactive: o.enabled, key: "stock" }),
    label: `${o.label ?? (o.enabled ? "Draw" : "Stock")} · ${o.count}`,
    enabled: o.enabled,
    title: o.title,
    onClick: o.onClick,
  });
}

export function slotEl(label: string, o: {
  lit?: boolean;
  small?: boolean;
  title?: string;
  onClick?: () => void;
} = {}): HTMLElement {
  const s = el("div", `slot${o.small ? " sm" : ""}${o.lit ? " lit" : ""}`, label);
  if (o.title) s.title = o.title;
  if (o.onClick) {
    s.tabIndex = 0;
    s.setAttribute("role", "button");
    s.onclick = o.onClick;
    keyClick(s);
  }
  return s;
}

export const RULES: Record<string, { title: string; items: string[] }> = {
  crazy8: {
    title: "8 Locos — how it plays",
    items: [
      "Match the top card by suit or rank. Any heart on a heart, any 7 on a 7.",
      "Every 8 is wild. Play it and call the suit that stays live.",
      "Draw one. If it matches, play it now or pass and keep it. If it does not, pass.",
      "An empty stock rebuilds from the discards, leaving the top card. First to zero cards wins the round.",
    ],
  },
  runs: {
    title: "Escaleras y Tríos — how it plays",
    items: [
      "Each turn: draw one (stock or discards), optionally build, then discard one.",
      "Trío: 3–4 cards of one rank, every suit different.",
      "Escalera: 3+ cards, one suit, unbroken sequence. Aces end runs (A-2-3, Q-K-A) and never bridge K-A-2.",
      "A card taken from the discards has to stay this turn, unless it goes down in a meld. Extend any meld, even a rival's.",
    ],
  },
  klondike: {
    title: "Solitaire — how it plays",
    items: [
      "Columns build down in alternating colours. Only a king opens an empty column.",
      "Foundations build up by suit, ace to king. All 52 home wins.",
      "Click a card, then a lit destination. Click the same card again to send it home, or to cancel.",
      "The stock turns one card and redeals without limit. Undo is free. Hover a buried card to read it.",
    ],
  },
  plaza: {
    title: "Plaza — how it plays",
    items: [
      "Four plazas, one per suit. On your turn place one card into any plaza with room.",
      "The 3rd card closes a plaza at once. A card of the plaza's own suit beats every foreigner; otherwise the highest rank takes it. Ties go to whoever committed first.",
      "Green takes the banner, red gifts it away. You still have to place.",
      "When the cards run out, most banners wins. Aces in their home plaza are unbeatable.",
    ],
  },
};
