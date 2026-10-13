import { send } from "../core/engine";
import { cardEl } from "../core/cards";
import { sfx } from "../core/sound";
import { RULES, button, drawPile, el, errorLine, finale, logEl, pileWrap, scoreRail, suitBadge, suitSheet } from "../core/ui";
import { cardName, rankOf, RANKS, SUIT_NAMES, type Crazy8State } from "../core/types";
import { mountTable, type Table } from "../core/table";
import { header, statusLine, type Shell } from "./shell";

export function mount(root: HTMLElement, shell: Shell): () => void {
  return mountTable<Crazy8State>({
    root,
    shell,
    rules: RULES.crazy8,
    withPlayers: true,
    beat: 520,
    open: (n, s) => send(`new|crazy8|${s}|${n}`) as Crazy8State,
    ai: (st) => st.phase !== "over" && st.turn !== st.you,
    over: (st) => st.phase === "over",
    render,
  });
}

function render(table: Table<Crazy8State>): void {
  const st = table.state;
  table.paint((view) => {
    const hand = st.players[st.you].hand ?? [];
    const mine = st.turn === st.you && st.phase !== "over";
    const pendingAt = st.pending == null ? -1 : hand.indexOf(st.pending);
    const drawn = st.mustPass ? st.last?.card ?? null : null;
    const legal = new Set(st.legal);

    view.append(
      header("8 Locos", "Match the top, or draw. First to empty a hand takes the round."),
      scoreRail(st.players.map((p, i) => ({
        name: p.name,
        stat: String(p.count),
        title: `${p.count} cards`,
        active: st.turn === i && st.phase !== "over",
        you: i === st.you,
      }))),
    );

    const felt = el("div", "felt");
    const mid = el("div", "row");
    const canDraw = mine && st.phase === "play" && st.canDraw;
    mid.append(drawPile({
      count: st.drawCount,
      enabled: canDraw,
      title: "Draw one card",
      onClick: () => table.commit("draw", { sound: sfx.draw }),
    }));
    mid.append(pileWrap({
      card: cardEl(st.top, { extra: st.last?.kind === "play" ? "just-played" : "" }),
      label: "Top",
    }));
    const liveWrap = el("div", "pile-wrap");
    const live = el("div", "live-suit");
    live.append(suitBadge(st.activeSuit));
    liveWrap.append(live, el("div", "pile-cap", "Live suit"));
    mid.append(liveWrap);
    felt.append(
      el("div", "felt-label", "Table"),
      mid,
      el("div", "hint", `Top is ${cardName(st.top)}. Match ${SUIT_NAMES[st.activeSuit]} or ${RANKS[rankOf(st.top)]} — any 8 is wild.`),
    );
    view.append(felt, statusLine(blurb(st), mine && st.phase === "play"), errorLine());

    const handEl = el("div", `hand fan${mine && st.phase === "play" ? " choosing" : ""}`);
    hand.forEach((c, i) => {
      const playable = mine && st.phase === "play" && legal.has(i);
      const marked = i === pendingAt ? "selected" : c === drawn ? "just-in" : "";
      const card = cardEl(c, {
        interactive: playable,
        extra: [playable || marked ? "" : "dim", playable ? "playable" : "", marked].filter(Boolean).join(" "),
      });
      if (playable) card.onclick = () => table.commit(`play|${i}`);
      handEl.append(card);
    });
    view.append(handEl);

    if (mine && st.phase === "play") {
      const ctl = el("div", "row");
      const pass = (): void => { table.commit("pass", { sound: sfx.flip }); };
      if (st.canPass && pendingAt >= 0) {
        ctl.append(
          button("Play drawn card", "primary", () => table.commit(`play|${pendingAt}`)),
          button("Pass", "", pass),
        );
      } else if (st.canPass) {
        ctl.append(button("Pass", "primary", pass));
      } else {
        const draw = button("Draw", legal.size ? "" : "primary", () => table.commit("draw", { sound: sfx.draw }));
        draw.disabled = !st.canDraw;
        ctl.append(draw);
      }
      view.append(ctl);
    }
    view.append(logEl(st.log));

    if (st.phase === "over") {
      const w = st.players[st.winner ?? 0];
      const youWon = st.winner === st.you;
      table.finish(finale(
        youWon ? "You win the round" : `${w.name} wins`,
        `${youWon ? "You went out first." : `${w.name} went out first.`} The round is worth ${st.roundPoints}. ${st.players.map((p) => `${p.name} ${p.score}`).join(" · ")}.`,
        "New game",
        () => table.restart(),
      ));
    }
  });

  const calling = st.phase === "choose_suit" && st.turn === st.you;
  const sheet = table.root.querySelector(".suit-overlay");
  if (calling && !sheet) table.root.append(suitSheet((suit) => table.commit(`suit|${suit}`)));
  else if (!calling && sheet) sheet.remove();
}

function blurb(st: Crazy8State): string {
  const who = st.players[st.turn]?.name ?? "They";
  if (st.phase === "over") {
    return st.winner === st.you
      ? `You take the round · +${st.roundPoints}.`
      : `${st.players[st.winner ?? 0].name} takes the round · +${st.roundPoints}.`;
  }
  if (st.turn !== st.you) {
    if (st.phase === "choose_suit") return `${who} is calling a suit…`;
    if (st.last?.kind === "draw" && st.last.player === st.turn) return `${who} drew a card…`;
    return `${who} is playing…`;
  }
  if (st.phase === "choose_suit") return "Call the suit your 8 leaves behind.";
  if (st.pending != null) return "That draw matches. Play it now, or pass and keep it.";
  if (st.mustPass) {
    return st.last?.card != null ? `${cardName(st.last.card)} does not match. Pass.` : "The stock is empty. Pass.";
  }
  if (st.legal.length === 0) return "Nothing matches. Draw one.";
  return "Play a lit card, or draw.";
}
