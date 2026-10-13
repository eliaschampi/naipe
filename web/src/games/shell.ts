// What every screen is allowed to ask of the shell around it.
import { el } from "../core/ui";

export interface Shell {
  home(): void;
  play(id: string): void;
  setActions(els: HTMLElement[]): void;
  setRules(title: string, items: string[]): void;
  onRestart(fn: () => void): void;
  enableNew(on: boolean): void;
}

export type Unmount = () => void;
export type Mount = (root: HTMLElement, shell: Shell) => Unmount;

export function header(title: string, sub: string): HTMLElement {
  const wrap = el("div");
  wrap.append(el("h1", "game-title", title), el("div", "game-sub", sub));
  return wrap;
}

export function statusLine(text: string, turn = false): HTMLElement {
  const d = el("div", "hint");
  d.setAttribute("role", "status");
  d.innerHTML = turn ? `<span class="turn-dot"></span>${text}` : text;
  return d;
}
