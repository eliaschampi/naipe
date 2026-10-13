import { el } from "../core/ui";
import type { Shell } from "./shell";

const GAMES = [
  { id: "crazy8", num: "01", name: "8 Locos", desc: "Crazy Eights — shed every card, call the suit with your wild 8s. You versus Nora, Silas and Iris." },
  { id: "runs", num: "02", name: "Escaleras y Tríos", desc: "Runs & sets rummy — draw, build trios and escaleras on the table, go out first." },
  { id: "klondike", num: "03", name: "Solitaire", desc: "Classic Klondike — seven columns, four foundations, unlimited redeals, full undo." },
  { id: "plaza", num: "04", name: "Plaza", desc: "An original: four plazas, three cards each. The third card decides — never gift a banner." },
];

export function mount(root: HTMLElement, shell: Shell): () => void {
  shell.enableNew(false);
  shell.setActions([]);
  shell.setRules("Naipe — a small card room", [
    "Four games, one deck, no accounts.",
    "Every game explains itself: open Rules on any table.",
    "Lit cards and lit piles are legal. The engine decides, the table shows.",
  ]);
  shell.onRestart(() => {});
  const go = (id: string): void => shell.play(id);
  const hero = el("div", "hero");
  const h = el("h1");
  h.innerHTML = "Four games,<br />one <em>deck.</em>";
  hero.append(
    h,
    el("p", "", "A small, fast card room. No accounts, no noise — pick a table and play. The rules live inside each game, one tap away."),
  );
  const list = el("div", "game-list");
  GAMES.forEach((g) => {
    const b = el("button", "game-entry");
    b.type = "button";
    b.append(
      el("span", "num", g.num),
      (() => {
        const t = el("div");
        t.append(el("div", "name", g.name), el("div", "desc", g.desc));
        return t;
      })(),
      el("span", "go", "→"),
    );
    b.onclick = () => go(g.id);
    list.append(b);
  });
  root.append(hero, list);
  return () => {};
}
