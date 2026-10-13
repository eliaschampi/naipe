import { cmd, send } from "../core/engine";
import { cardEl } from "../core/cards";
import { sfx } from "../core/sound";
import { RULES, button, drawPile, el, errorLine, finale, logEl, pileWrap, scoreRail } from "../core/ui";
import { type CardId, type RunsState } from "../core/types";
import { mountTable, type Table } from "../core/table";
import { header, statusLine, type Shell } from "./shell";

export function mount(root: HTMLElement, shell: Shell): () => void {
  let sel = new Set<number>();
  let verdict = "";
  let legalMeld = false;

  function recheck(st: RunsState): void {
    const hand = st.players[st.you].hand ?? [];
    [...sel].filter((i) => i >= hand.length).forEach((i) => sel.delete(i));
    if (sel.size < 3) {
      legalMeld = false;
      verdict = sel.size === 0 ? "" : "Three or more cards make a meld. One card extends or discards.";
      return;
    }
    try {
      const r = cmd(`check|${[...sel].join(",")}`);
      const d = r.data as { valid?: boolean; kind?: string | null; reason?: string | null } | null;
      legalMeld = Boolean(d?.valid);
      verdict = legalMeld
        ? `A clean ${d?.kind === "trio" ? "trío" : "escalera"}. Lay it down.`
        : (d?.reason ?? "Not a combination.");
    } catch {
      legalMeld = false;
      verdict = "";
    }
  }

  function clearSel(): void {
    sel.clear();
    verdict = "";
    legalMeld = false;
  }

  return mountTable<RunsState>({
    root,
    shell,
    rules: RULES.runs,
    withPlayers: true,
    beat: 560,
    open: (n, s) => send(`new|runs|${s}|${n}`) as RunsState,
    ai: (st) => st.phase !== "over" && st.turn !== st.you,
    over: (st) => st.phase === "over",
    onKey(e, table) {
      if (e.key === "Escape" && sel.size > 0) {
        clearSel();
        table.say("");
        table.refresh();
      }
    },
    render(table) {
      const st = table.state;
      const hand = st.players[st.you].hand ?? [];
      [...sel].filter((i) => i >= hand.length).forEach((i) => sel.delete(i));
      const mine = st.turn === st.you && st.phase !== "over";
      const building = mine && st.phase === "build";
      const top = st.discard[st.discard.length - 1];

      table.paint((view) => {
        view.append(
          header("Escaleras y Tríos", "Draw one, build, then discard. First out takes the deadwood."),
          scoreRail(st.players.map((p, i) => ({
            name: p.name,
            stat: String(p.count),
            title: `${p.count} cards · deadwood ${p.deadwood ?? 0}`,
            active: st.turn === i && st.phase !== "over",
            you: i === st.you,
          }))),
        );

        const felt = el("div", "felt");
        const mid = el("div", "row");
        const canTake = mine && st.phase === "draw";
        mid.append(drawPile({
          count: st.drawCount,
          enabled: canTake,
          title: "Draw from the stock",
          onClick: () => table.commit("drawstock", { sound: sfx.draw, then: clearSel }),
        }));
        if (top !== undefined) {
          const card = cardEl(top, { interactive: canTake });
          mid.append(pileWrap({
            card,
            label: canTake ? "Take" : "Discard",
            enabled: canTake,
            title: "Take this card. It has to stay this turn.",
            onClick: () => table.commit("drawdiscard", { sound: sfx.draw, then: clearSel }),
          }));
        }
        felt.append(el("div", "felt-label", "Draw"), mid);
        view.append(felt);

        if (st.melds.length > 0) {
          const sec = el("div", "felt");
          sec.append(el("div", "felt-label", "Table"));
          const melds = el("div", "melds");
          const only = sel.size === 1 ? [...sel][0] : -1;
          st.melds.forEach((m, mi) => {
            const fits = building && only >= 0 && (st.extends[mi] ?? []).includes(only);
            const box = el("div", `meld${building ? " hot" : ""}${fits ? " lit" : ""}`);
            box.append(el("div", "meld-tag", `${m.kind === "trio" ? "Trío" : "Escalera"} · ${st.players[m.owner].name}`));
            m.cards.forEach((c) => box.append(cardEl(c, { small: true })));
            if (building) {
              box.onclick = () => {
                if (sel.size !== 1) {
                  table.say("Select one card, then the meld it extends.");
                  return;
                }
                const i = [...sel][0];
                table.commit(`extend|${m.id}|${i}`, { then: clearSel });
              };
            }
            melds.append(box);
          });
          sec.append(melds);
          view.append(sec);
        }

        const turnText = st.phase === "over"
          ? st.winner === st.you ? "You went out." : `${st.players[st.winner ?? 0].name} went out.`
          : !mine ? `${st.players[st.turn].name} is playing…`
          : st.phase === "draw" ? "Draw one card — from the stock, or take the discard."
          : "Build if you want, then discard one card.";
        view.append(statusLine(turnText, building || (mine && st.phase === "draw")), el("div", "hint", verdict), errorLine());

        const handEl = el("div", "hand fan");
        hand.forEach((c, i) => {
          const locked = building && st.taken === c && hand.length > 1;
          const on = building;
          const card = cardEl(c, {
            interactive: on,
            extra: [
              on ? (sel.has(i) ? "selected playable" : "playable") : "dim",
              locked ? "locked" : "",
            ].filter(Boolean).join(" "),
          });
          if (locked) card.title = "Taken from the discards — it stays unless you meld it";
          if (on) {
            card.onclick = () => {
              if (sel.has(i)) sel.delete(i);
              else sel.add(i);
              sfx.flip();
              recheck(st);
              table.refresh();
            };
          }
          handEl.append(card);
        });
        view.append(handEl);

        if (mine) {
          const takenId = st.taken;
          const sole = sel.size === 1 ? hand[[...sel][0]] : undefined;
          const lockedDiscard = sole !== undefined && takenId === sole && hand.length > 1;
          const ctl = el("div", "row");
          if (building) {
            const lay = button("Lay meld", "primary", () => {
              table.commit(`meld|${[...sel].sort((a, b) => a - b).join(",")}`, { then: clearSel });
            });
            const disc = button("Discard", "", () => {
              table.commit(`discard|${[...sel][0]}`, { then: clearSel });
            });
            lay.disabled = !legalMeld;
            disc.disabled = !(sel.size === 1 && !lockedDiscard);
            const clear = button("Clear", "ghost", () => {
              clearSel();
              table.refresh();
            });
            clear.disabled = sel.size === 0;
            ctl.append(lay, disc, clear);
          }
          const bySuit = button("Sort suit", "ghost small", () => reorder(table, "sort|suit"));
          const byRank = button("Sort rank", "ghost small", () => reorder(table, "sort|rank"));
          ctl.append(bySuit, byRank);
          if (building && sel.size === 1) {
            const i = [...sel][0];
            ctl.append(
              button("←", "ghost small", () => reorder(table, `arrange|${i}|${Math.max(0, i - 1)}`)),
              button("→", "ghost small", () => reorder(table, `arrange|${i}|${Math.min(hand.length - 1, i + 1)}`)),
            );
          }
          view.append(ctl);
        }
        view.append(el("div", "hint", `Deadwood — ${st.players.map((p) => `${p.name} ${p.deadwood ?? 0}`).join(" · ")}`));
        view.append(logEl(st.log));

        if (st.phase === "over") {
          const w = st.players[st.winner ?? 0];
          const youWon = st.winner === st.you;
          table.finish(finale(
            youWon ? "You go out" : `${w.name} goes out`,
            youWon ? "You emptied your hand first." : `${w.name} emptied first.`,
            "New round",
            () => table.restart(),
          ));
        }
      });

      function reorder(table: Table<RunsState>, raw: string): void {
        const keep = [...sel].map((i) => hand[i]).filter((c): c is CardId => c !== undefined);
        table.commit(raw, {
          sound: sfx.flip,
          then: () => {
            const next = table.state.players[table.state.you].hand ?? [];
            sel = new Set(keep.map((id) => next.indexOf(id)).filter((i) => i >= 0));
            recheck(table.state);
          },
        });
      }
    },
  });
}
