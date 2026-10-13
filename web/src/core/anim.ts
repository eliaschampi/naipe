// One FLIP helper. Identity is the card and which face is showing, so a
// selection change does not pretend the card is new, and a move flies.
export function flip(container: HTMLElement, mutate: () => void, ms = 220): void {
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
    mutate();
    return;
  }
  const before = new Map<string, DOMRect>();
  container.querySelectorAll<HTMLElement>("[data-card]").forEach((el) => {
    before.set(cardKey(el), el.getBoundingClientRect());
  });
  mutate();
  container.querySelectorAll<HTMLElement>("[data-card]").forEach((el) => {
    const from = before.get(cardKey(el));
    if (!from) {
      el.animate(
        [
          { opacity: 0, transform: "translateY(6px)" },
          { opacity: 1, transform: "none" },
        ],
        { duration: ms, easing: "cubic-bezier(.22,.8,.3,1)" },
      );
      return;
    }
    const to = el.getBoundingClientRect();
    const dx = from.left - to.left;
    const dy = from.top - to.top;
    if (dx || dy) {
      el.animate(
        [{ transform: `translate(${dx}px,${dy}px)` }, { transform: "none" }],
        { duration: ms, easing: "cubic-bezier(.22,.8,.3,1)" },
      );
    }
  });
}

function cardKey(el: HTMLElement): string {
  return `${el.dataset.card ?? ""}:${el.dataset.face ?? "u"}`;
}

export const delay = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms));
