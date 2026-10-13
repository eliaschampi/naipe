import "./styles/theme.css";
import { loadEngine } from "./core/engine";
import { el, overlay, rulesBody, soundButton, topbar } from "./core/ui";
import { soundOn, toggleSound } from "./core/sound";
import type { Shell } from "./games/shell";
import { mount as mountHome } from "./games/home";
import { mount as mountCrazy8 } from "./games/crazy8";
import { mount as mountRuns } from "./games/runs";
import { mount as mountKlondike } from "./games/klondike";
import { mount as mountPlaza } from "./games/plaza";

const GAMES: Record<string, (root: HTMLElement, shell: Shell) => () => void> = {
  home: mountHome,
  crazy8: mountCrazy8,
  runs: mountRuns,
  klondike: mountKlondike,
  plaza: mountPlaza,
};

async function boot(): Promise<void> {
  const app = document.getElementById("app")!;
  const barHost = el("div");
  const stage = el("div", "stage");
  app.append(barHost, stage);

  let unmount: () => void = () => {};
  let restartFn: () => void = () => {};
  let canNew = false;
  let rules = { title: "Naipe", items: [] as string[] };
  let actions: HTMLElement[] = [];
  let view = "home";

  const snd = soundButton(
    () => (soundOn() ? "♪" : "∅"),
    () => {
      toggleSound();
    },
  );

  function drawBar(): void {
    barHost.innerHTML = "";
    barHost.append(
      topbar({
        onHome: () => navigate("home"),
        onRules: () => stage.append(overlay(rules.title, rulesBody(rules.items), () => {})),
        onNew: () => restartFn(),
        extra: actions,
        soundBtn: snd,
        canNew,
      }),
    );
  }

  const shell: Shell = {
    home: () => navigate("home"),
    play: (id: string) => navigate(id),
    setActions: (els: HTMLElement[]) => {
      actions = els;
      drawBar();
    },
    setRules: (title: string, items: string[]) => {
      rules = { title, items };
    },
    onRestart: (fn: () => void) => {
      restartFn = fn;
    },
    enableNew: (on: boolean) => {
      canNew = on;
      const b = barHost.querySelector<HTMLButtonElement>('button[data-role="new"]');
      if (b) b.disabled = !on;
    },
  };

  function navigate(next: string): void {
    if (!GAMES[next]) next = "home";
    view = next;
    unmount();
    document.querySelectorAll(".overlay").forEach((o) => o.remove());
    stage.innerHTML = "";
    actions = [];
    window.scrollTo(0, 0);
    unmount = GAMES[view](stage, shell);
    drawBar();
  }

  stage.append(el("p", "hint", "Shuffling…"));
  try {
    await loadEngine();
  } catch {
    stage.innerHTML = "";
    stage.append(el("p", "err", "The card engine failed to load. Serve the app over http (vite dev/preview) and reload."));
    return;
  }
  navigate("home");
}

void boot();
