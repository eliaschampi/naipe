import { send } from "../core/engine";
import { cardEl } from "../core/cards";
import { sfx } from "../core/sound";
import { RULES, el, errorLine, finale, logEl, scoreRail, slotEl } from "../core/ui";
import { SUIT_NAMES, SUITS, type PlazaState } from "../core/types";
import { mountTable, type Table } from "../core/table";
import { header, statusLine, type Shell } from "./shell";

const TONE: Record<string, string> = {
  win: "Takes",
  gift: "Gifts",
  lead: "Leads",
  trail: "Trails",
};

export function mount(root: HTMLElement, shell: Shell): () => void {
  let sel: number | null = null;
  let seenBanner: string | null = null;

  return mountTable<PlazaState>({
    root,
    shell,
    rules: RULES.plaza,
    withPlayers: true,
    beat: 560,
    open: (n, s) => send(`new|plaza|${s}|${n}`) as PlazaState,
    ai: (st) => st.phase !== "over" && st.turn !== st.you,
    over: (st) => st.phase === "over",
    onKey(e, table) {
      if (e.key === "Escape" && sel !== null) {
        sel = null;
        table.say("");
        table.refresh();
      }
    },
    render(table) {
      const st = table.state;
      const hand = st.players[st.you].hand ?? [];
      if (sel !== null && sel >= hand.length) sel = null;
      const mine = st.turn === st.you && st.phase === "play";

      table.paint((view) => {
        view.append(
          header("Plaza", "Four squares. The third card closes one and awards its banner."),
          scoreRail(st.players.map((p, i) => ({
            name: p.name,
            stat: `★${p.banners ?? 0} · ${p.count}`,
            title: `${p.banners ?? 0} banners, ${p.count} cards`,
            active: st.turn === i && st.phase !== "over",
            you: i === st.you,
          }))),
        );

        if (st.resolution) {
          const r = st.resolution;
          const key = JSON.stringify(r.cards);
          const fresh = key !== seenBanner;
          seenBanner = key;
          view.append(el(
            "div",
            `banner-flash${fresh ? "" : " still"}`,
            `${st.players[r.winner].name} takes the ${SUIT_NAMES[r.plaza]} banner ${SUITS[r.plaza]}`,
          ));
        } else {
          seenBanner = null;
        }

        const felt = el("div", "felt");
        felt.append(el("div", "felt-label", `Plazas · the 3rd card closes · stock ${st.stockCount}`));
        const grid = el("div", "plazas");
        st.plazas.forEach((pz, pi) => {
          const tag = sel !== null && mine ? st.preview[sel]?.[pi] : "";
          const open = pz.cards.length < 3 && tag !== "closed";
          const ready = mine && sel !== null && open;
          const box = el("div", `plaza${ready ? ` ready tone-${tag}` : ""}`);
          const lead = pz.leader !== null ? st.players[pz.leader].name : "open";
          const head = el("div", "p-head");
          head.append(
            el("span", "", `${SUITS[pi]} ${SUIT_NAMES[pi]}`),
            el("span", "", `${pz.cards.length}/3 · ${lead}`),
          );
          const cards = el("div", "p-cards");
          pz.cards.forEach(({ c, o }) => {
            const cd = cardEl(c, { small: true, extra: o === st.you ? "mine" : "" });
            cd.title = st.players[o].name;
            cards.append(cd);
          });
          for (let k = pz.cards.length; k < 3; k++) cards.append(slotEl("·", { small: true }));
          box.append(head, cards);
          if (ready && tag && TONE[tag]) box.append(el("div", `tone tone-${tag}`, TONE[tag]));
          if (ready) {
            box.onclick = () => {
              const i = sel;
              if (i === null) return;
              table.commit(`play|${i}|${pi}`, { then: () => { sel = null; } });
            };
          }
          grid.append(box);
        });
        felt.append(grid);
        view.append(felt);

        const text = st.phase === "over"
          ? st.tie
            ? "Level on banners."
            : st.winner === st.you
              ? "You hold the most banners."
              : `${st.players[st.winner ?? 0].name} holds the most banners.`
          : mine
            ? sel === null
              ? "Choose a card, then a plaza. Green takes the banner, red gifts it."
              : "Choose a plaza. Green takes it, red gives it away."
            : `${st.players[st.turn].name} is placing…`;
        view.append(statusLine(text, mine), errorLine());

        const handEl = el("div", "hand fan");
        hand.forEach((c, i) => {
          const on = mine;
          const card = cardEl(c, {
            interactive: on,
            extra: on ? (sel === i ? "selected playable" : "playable") : "dim",
          });
          if (on) {
            card.onclick = () => {
              sel = sel === i ? null : i;
              sfx.flip();
              table.say("");
              table.refresh();
            };
          }
          handEl.append(card);
        });
        view.append(handEl, logEl(st.log));

        if (st.phase === "over") {
          const w = st.players[st.winner ?? 0];
          const youWon = st.winner === st.you && !st.tie;
          const title = st.tie ? "Honours shared" : youWon ? "You win" : `${w.name} wins`;
          const line = st.tie
            ? `Dead level — ${st.players.map((p) => `${p.name} ${p.banners}`).join(" · ")}.`
            : `${youWon ? "You take the day." : `${w.name} takes the day.`} Banners — ${st.players.map((p) => `${p.name} ${p.banners}`).join(" · ")}.`;
          table.finish(finale(title, line, "New game", () => table.restart()));
        }
      });
    },
  });
}
