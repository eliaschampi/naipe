# Naipe — four card games, one deck

8 Locos (Crazy Eights) · Escaleras y Tríos (runs & sets) · Klondike Solitaire · Plaza (original).
Local only. No accounts, no network, no backend.

## Layout

- `engine/` — Rust, the single source of truth for rules, state, and AI. Zero dependencies.
- `web/` — TypeScript + CSS presentation only. It never decides legality; it renders engine state.

The boundary is raw WebAssembly: pipe-delimited commands in, a JSON state document out.

## Run it

```sh
# 1. Build the engine
cd engine
cargo test                                        # 64 rule tests
cargo build --target wasm32-unknown-unknown --release

# 2. Serve the app
cd ../web
npm install
npm run sync:wasm                                 # copy the engine bundle into public/
npm run dev                                       # → http://localhost:5173
```

Production: `npm run build && npm run preview`.

## Controls (every table)

- Lit cards are legal — the engine decides, the table shows.
- **8 Locos**: click a lit card, or Draw. A matching draw can be played or kept with Pass; anything else must be passed. An 8 opens the suit picker.
- **Runs & Sets**: draw from stock or discards, click cards to select, Lay meld / click a table meld to extend / Discard.
- **Solitaire**: click a card, then a lit destination. Click that card again to send it home. Undo is free.
- **Plaza**: pick a card, then a plaza. Green takes, red gifts.
