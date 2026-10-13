import { cmd, send } from "../core/engine";
import { cardEl } from "../core/cards";
import { sfx } from "../core/sound";
import { RULES, button, el, errorLine, finale, pileWrap, slotEl } from "../core/ui";
import { redSuit, SUITS, type KlondikeState } from "../core/types";
import { mountTable, type Table } from "../core/table";
import { header, statusLine, type Shell } from "./shell";

interface Sel {
  src: string;
  idx: number;
}

export function mount(root: HTMLElement, shell: Shell): () => void {
  let sel: Sel | null = null;
  let targets: string[] = [];

  function clear(): void {
    sel = null;
    targets = [];
  }

  function ask(src: string, idx: number): string[] {
    try {
      const r = cmd(`targets|${src}|${idx}`);
      if (!r.ok) return [];
      return (r.data as { targets?: string[] } | null)?.targets ?? [];
    } catch {
      return [];
    }
  }

  return mountTable<KlondikeState>({
    root,
    shell,
    rules: RULES.klondike,
    open: (_n, s) => send(`new|klondike|${s}|1`) as KlondikeState,
    ai: () => false,
    over: (st) => st.won,
    onKey(e, table) {
      if (e.key === "Escape" && sel) {
        clear();
        table.say("");
        table.refresh();
      } else if (e.key === "Enter" && sel && targets.some((t) => t.startsWith("f"))) {
        e.preventDefault();
        const cur = sel;
        table.commit(`auto|${cur.src}|${cur.idx}`, { then: clear });
      }
    },
    render(table) {
      const st = table.state;

      function pick(src: string, idx: number): void {
        if (st.won) return;
        if (sel && sel.src === src && sel.idx === idx) {
          if (targets.some((t) => t.startsWith("f"))) {
            table.commit(`auto|${src}|${idx}`, { then: clear });
          } else {
            clear();
            table.say("");
            table.refresh();
          }
          return;
        }
        if (sel && sel.src !== src && targets.includes(src)) {
          table.commit(`move|${sel.src}|${sel.idx}|${src}`, { then: clear });
          return;
        }
        const next = ask(src, idx);
        if (next.length === 0) {
          table.say("That card has nowhere to go.");
          return;
        }
        sel = { src, idx };
        targets = next;
        sfx.flip();
        table.say("");
        table.refresh();
      }

      function drop(loc: string): void {
        if (st.won || !sel) return;
        if (targets.includes(loc)) {
          table.commit(`move|${sel.src}|${sel.idx}|${loc}`, { then: clear });
          return;
        }
        table.say("That spot can't take this card.");
      }

      function turn(): void {
        if (st.won || (st.stockCount === 0 && st.wasteCount === 0)) return;
        table.commit("draw", {
          sound: st.stockCount > 0 ? sfx.draw : sfx.flip,
          then: clear,
        });
      }

      table.paint((view) => {
        const holding = sel !== null && !st.won;
        view.append(
          header("Solitaire", "Seven columns, four foundations, unlimited redeals."),
          statusLine(st.won
            ? `Solved in ${st.moves} moves.`
            : holding
              ? `Move ${st.moves}. A lit spot can take it — click the card again to send it home.`
              : `Move ${st.moves}. Click a card, then a lit spot.`),
          errorLine(),
        );

        const felt = el("div", "kl-table felt");
        felt.append(el("div", "felt-label", "Table"));
        const top = el("div", "kl-top");
        const canTurn = !st.won && (st.stockCount > 0 || st.wasteCount > 0);
        if (st.stockCount > 0) {
          top.append(pileWrap({
            card: cardEl(0, { faceDown: true, interactive: canTurn, key: "stock" }),
            label: `Stock · ${st.stockCount}`,
            enabled: canTurn,
            title: "Turn one card",
            onClick: turn,
          }));
        } else {
          top.append(pileWrap({
            card: slotEl("↻", {
              lit: canTurn,
              title: canTurn ? "Turn the waste back over" : "Stock empty",
              onClick: canTurn ? turn : undefined,
            }),
            label: "Stock · 0",
            enabled: canTurn,
            onClick: canTurn ? turn : undefined,
          }));
        }

        const waste = el("div", "waste");
        if (st.waste.length === 0) waste.append(slotEl("·"));
        st.waste.forEach((c, i) => {
          const last = i === st.waste.length - 1;
          const chosen = last && sel?.src === "w";
          const card = cardEl(c, {
            interactive: last && !st.won,
            extra: last ? (chosen ? "selected playable" : "playable") : "",
          });
          if (last && !st.won) card.onclick = () => pick("w", 0);
          waste.append(card);
        });
        const wasteWrap = el("div", "pile-wrap");
        wasteWrap.append(waste, el("div", "pile-cap", `Waste · ${st.wasteCount}`));
        top.append(wasteWrap, el("div", "spacer"));

        const founds = el("div", "kl-founds");
        st.foundations.forEach((f, fi) => {
          const loc = `f${fi}`;
          const cap = el("div", `pile-cap${redSuit(fi) ? " red" : ""}`, SUITS[fi]);
          let face: HTMLElement;
          if (f.length === 0) {
            const lit = targets.includes(loc);
            face = slotEl("A", { lit, title: "Aces start a foundation", onClick: lit ? () => drop(loc) : undefined });
          } else {
            const chosen = sel?.src === loc;
            const aimed = sel !== null && targets.includes(loc) && sel.src !== loc;
            face = cardEl(f[f.length - 1], {
              interactive: !st.won,
              extra: chosen ? "selected playable" : aimed ? "targetable playable" : "playable",
            });
            if (!st.won) face.onclick = () => pick(loc, 0);
          }
          founds.append(pileWrap({ card: face, label: cap }));
        });
        top.append(founds);
        felt.append(top);

        const grid = el("div", "kl-grid");
        st.tableau.forEach((col, t) => {
          const loc = `t${t}`;
          const column = el("div", "kl-col");
          if (col.length === 0) {
            const lit = targets.includes(loc);
            column.append(slotEl("K", {
              lit,
              title: "Kings open an empty column",
              onClick: lit ? () => drop(loc) : undefined,
            }));
          }
          col.forEach(({ c, up }, idx) => {
            if (!up) {
              column.append(cardEl(c, { faceDown: true }));
              return;
            }
            const chosen = sel?.src === loc && sel.idx === idx;
            const aimed = sel !== null && sel.src !== loc && targets.includes(loc);
            const card = cardEl(c, {
              interactive: !st.won,
              extra: chosen ? "selected playable" : aimed ? "targetable playable" : "playable",
            });
            if (!st.won) card.onclick = () => pick(loc, idx);
            column.append(card);
          });
          grid.append(column);
        });
        felt.append(grid);
        view.append(felt);

        const ctl = el("div", "row");
        const undo = button("Undo", "", () => table.commit("undo", { sound: sfx.flip, then: clear }));
        undo.disabled = !st.canUndo;
        const finish = button("Finish", "", () => table.commit("autoall", { then: clear }));
        finish.disabled = !st.openBoard || st.won;
        finish.title = "Sweep every open card home";
        ctl.append(undo, finish, button("New deal", "primary", () => table.restart()));
        view.append(ctl);

        if (st.won) {
          table.finish(finale(
            "Solved",
            `All four foundations are home, in ${st.moves} moves.`,
            "New deal",
            () => table.restart(),
          ));
        }
      });
    },
  });
}
