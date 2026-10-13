// One card face for every table. Full-size number cards use a pip map;
// small cards (melds, plazas) keep a single suit so they stay readable.
import { keyClick } from "./ui";
import { isRed, rankOf, RANKS, suitOf, SUITS, SUIT_NAMES, type CardId } from "./types";

/** Pip centers, as percentages of the card. The key is the rank index:
    1 is a two, 9 is a ten. Spots sit inside the index bands. */
const PIPS: Record<number, [number, number][]> = {
  1: [[50, 32], [50, 68]],
  2: [[50, 30], [50, 50], [50, 70]],
  3: [[35, 32], [65, 32], [35, 68], [65, 68]],
  4: [[35, 30], [65, 30], [50, 50], [35, 70], [65, 70]],
  5: [[35, 30], [65, 30], [35, 50], [65, 50], [35, 70], [65, 70]],
  6: [[35, 28], [65, 28], [50, 39], [35, 50], [65, 50], [35, 72], [65, 72]],
  7: [[35, 28], [65, 28], [50, 38], [35, 49], [65, 49], [50, 62], [35, 72], [65, 72]],
  8: [[35, 28], [65, 28], [35, 41], [65, 41], [50, 50], [35, 59], [65, 59], [35, 72], [65, 72]],
  9: [[35, 27], [65, 27], [35, 41], [50, 41], [65, 41], [35, 59], [50, 59], [65, 59], [35, 73], [65, 73]],
};

export interface CardOpts {
  small?: boolean;
  faceDown?: boolean;
  extra?: string;
  /** Can be clicked or keyed. Dim status cards stay out of the tab order. */
  interactive?: boolean;
  /** FLIP identity. Anonymous backs must not reuse a real card id. */
  key?: string;
}

export function cardEl(id: CardId, opts: CardOpts = {}): HTMLElement {
  const d = document.createElement("div");
  d.dataset.card = opts.key ?? String(id);
  d.dataset.face = opts.faceDown ? "d" : "u";
  const extra = opts.extra ? ` ${opts.extra}` : "";
  if (opts.faceDown) {
    d.className = `card back${opts.small ? " sm" : ""}${extra}`;
  } else {
    const s = suitOf(id);
    const r = rankOf(id);
    d.className = `card${isRed(id) ? " red" : ""}${opts.small ? " sm" : ""}${extra}`;
    d.setAttribute("aria-label", `${RANKS[r]} of ${SUIT_NAMES[s]}`);
    d.innerHTML = corners(r, s) + face(r, s, Boolean(opts.small));
  }
  if (opts.interactive) {
    d.tabIndex = 0;
    d.setAttribute("role", "button");
    keyClick(d);
  }
  return d;
}

function corners(r: number, s: number): string {
  const mark = `${RANKS[r]}<span class="suit">${SUITS[s]}</span>`;
  return `<span class="corner tl">${mark}</span><span class="corner br">${mark}</span>`;
}

function face(r: number, s: number, small: boolean): string {
  const glyph = SUITS[s];
  if (!small && r >= 10) {
    return `<span class="court"><b>${RANKS[r]}</b><i>${glyph}</i></span>`;
  }
  const spots = !small ? PIPS[r] : undefined;
  if (spots) {
    const marks = spots.map(([x, y]) => `<i style="left:${x}%;top:${y}%">${glyph}</i>`).join("");
    return `<span class="pips">${marks}</span>`;
  }
  return `<span class="pip">${glyph}</span>`;
}
