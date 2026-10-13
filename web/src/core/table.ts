// The lifecycle every table shares: deal, command, error, opponent beats, one win sheet.
import { delay, flip } from "./anim";
import { seed, send } from "./engine";
import { sfx } from "./sound";
import { playerSelect } from "./ui";
import type { Shell } from "../games/shell";

export interface CommitOpts {
  sound?: () => void;
  /** Runs only after the engine accepts the command, before the paint. */
  then?: () => void;
}

export interface Table<S> {
  readonly state: S;
  readonly players: number;
  readonly root: HTMLElement;
  commit(raw: string, opts?: CommitOpts): boolean;
  /** A line under the table. Errors buzz; ordinary hints do not. */
  say(message: string, bad?: boolean): void;
  refresh(): void;
  restart(): void;
  finish(sheet: HTMLElement): void;
  paint(draw: (view: HTMLElement) => void): void;
}

interface TableOpts<S> {
  root: HTMLElement;
  shell: Shell;
  rules: { title: string; items: string[] };
  /** Show the 2/3/4 player control. Solitaire leaves this off. */
  withPlayers?: boolean;
  open: (players: number, seed: number) => S;
  ai: (state: S) => boolean;
  over: (state: S) => boolean;
  beat?: number;
  onKey?: (e: KeyboardEvent, table: Table<S>) => void;
  render: (table: Table<S>) => void;
}

export function mountTable<S>(opts: TableOpts<S>): () => void {
  let state = opts.open(opts.withPlayers ? 3 : 1, seed());
  let players = opts.withPlayers ? 3 : 1;
  let alive = true;
  let busy = false;
  let run = 0;
  let error = "";
  let errorBad = false;
  let celebrated = false;

  const api: Table<S> = {
    get state() { return state; },
    get players() { return players; },
    root: opts.root,
    commit,
    say,
    refresh,
    restart,
    finish,
    paint,
  };

  opts.shell.setRules(opts.rules.title, opts.rules.items);
  opts.shell.enableNew(true);
  opts.shell.onRestart(() => start(true));
  if (opts.withPlayers) {
    opts.shell.setActions([playerSelect(players, (n) => {
      players = n;
      start(true);
    })]);
  } else {
    opts.shell.setActions([]);
  }

  const onKey = (e: KeyboardEvent): void => {
    if (!alive) return;
    const t = e.target;
    if (t instanceof HTMLElement && (t.tagName === "INPUT" || t.tagName === "SELECT" || t.tagName === "TEXTAREA")) return;
    if (opts.root.querySelector(".overlay")) return;
    opts.onKey?.(e, api);
  };
  document.addEventListener("keydown", onKey);

  refresh();
  pump();

  function refresh(): void {
    opts.render(api);
  }

  function say(message: string, bad = false): void {
    error = message;
    errorBad = bad;
    if (bad) sfx.bad();
    applyError();
  }

  function applyError(): void {
    const node = opts.root.querySelector<HTMLElement>(".err");
    if (!node) return;
    node.textContent = error;
    node.classList.toggle("bad", errorBad && error.length > 0);
  }

  function commit(raw: string, o?: CommitOpts): boolean {
    if (!alive || busy) return false;
    try {
      state = send(raw) as S;
      error = "";
      errorBad = false;
      o?.then?.();
      (o?.sound ?? sfx.place)();
      refresh();
      pump();
      return true;
    } catch (e) {
      say((e as Error).message || "Illegal move.", true);
      return false;
    }
  }

  function paint(draw: (view: HTMLElement) => void): void {
    const view = ensureView();
    const hand = view.querySelector<HTMLElement>(".hand");
    const prev = hand?.scrollLeft ?? 0;
    if (!opts.over(state)) {
      celebrated = false;
      opts.root.querySelectorAll(".overlay.finale").forEach((n) => n.remove());
    }
    flip(view, () => {
      view.replaceChildren();
      draw(view);
      applyError();
    });
    holdHand(view.querySelector<HTMLElement>(".hand"), prev);
  }

  function finish(sheet: HTMLElement): void {
    if (celebrated || !opts.over(state)) return;
    celebrated = true;
    sfx.win();
    opts.root.append(sheet);
  }

  function restart(): void {
    start(true);
  }

  function start(announce: boolean): void {
    run += 1;
    busy = false;
    error = "";
    errorBad = false;
    celebrated = false;
    opts.root.querySelectorAll(".overlay").forEach((n) => n.remove());
    state = opts.open(players, seed());
    if (announce) sfx.deal();
    refresh();
    pump();
  }

  function pump(): void {
    const token = ++run;
    if (!alive || opts.over(state) || !opts.ai(state)) {
      if (token === run) busy = false;
      return;
    }
    busy = true;
    void (async () => {
      try {
        while (alive && token === run && !opts.over(state) && opts.ai(state)) {
          await delay(opts.beat ?? 520);
          if (!alive || token !== run) return;
          state = send("ai") as S;
          sfx.place();
          refresh();
        }
      } catch (e) {
        if (token === run) say((e as Error).message || "The opponent could not move.", true);
      } finally {
        if (token === run) busy = false;
      }
    })();
  }

  function ensureView(): HTMLElement {
    let view = opts.root.querySelector<HTMLElement>(":scope > .table-view");
    if (!view) {
      view = document.createElement("div");
      view.className = "table-view";
      opts.root.append(view);
    }
    return view;
  }

  return () => {
    alive = false;
    run += 1;
    document.removeEventListener("keydown", onKey);
  };
}

function holdHand(hand: HTMLElement | null, prev: number): void {
  if (!hand) return;
  const card = hand.querySelector<HTMLElement>(".card.selected, .card.just-in");
  if (!card) {
    hand.scrollLeft = prev;
    return;
  }
  const left = card.offsetLeft - 12;
  const right = card.offsetLeft + card.offsetWidth + 12;
  if (left < prev) hand.scrollLeft = Math.max(0, left);
  else if (right > prev + hand.clientWidth) hand.scrollLeft = right - hand.clientWidth;
  else hand.scrollLeft = prev;
}
