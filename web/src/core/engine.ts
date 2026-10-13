// Single bridge to the Rust rules engine (compiled to raw WASM, no bindings).
// Protocol: write a pipe-delimited command into shared memory, call dispatch,
// read the JSON reply. Every reply carries the full next state.
import type { GameState, Reply } from "./types";

type Exports = {
  memory: WebAssembly.Memory;
  input_ptr: () => number;
  input_capacity: () => number;
  output_ptr: () => number;
  dispatch: (len: number) => number;
};

let ex: Exports | null = null;
let mem8: Uint8Array | null = null;

function refreshView(): Uint8Array {
  mem8 = new Uint8Array(ex!.memory.buffer);
  return mem8;
}

export async function loadEngine(): Promise<void> {
  if (ex) return;
  const base =
    (import.meta as unknown as { env?: { BASE_URL?: string } }).env?.BASE_URL ?? "./";
  const url = `${base}naipe_engine.wasm`;
  const bytes = await (await fetch(url)).arrayBuffer();
  const mod = await WebAssembly.instantiate(bytes, {});
  ex = mod.instance.exports as unknown as Exports;
  refreshView();
}

function writeCmd(cmd: string): number {
  const mem = refreshView();
  const ptr = ex!.input_ptr();
  const cap = ex!.input_capacity();
  const enc = new TextEncoder().encode(cmd.slice(0, cap));
  mem.set(enc, ptr);
  return enc.length;
}

function readReply(len: number): string {
  const mem = refreshView();
  const ptr = ex!.output_ptr();
  return new TextDecoder().decode(mem.slice(ptr, ptr + len));
}

export function cmd(raw: string): Reply {
  if (!ex) throw new Error("engine not loaded");
  const n = writeCmd(raw);
  const outLen = ex.dispatch(n);
  const json = readReply(outLen);
  const parsed = JSON.parse(json) as Reply;
  if (!parsed.state) (parsed as Reply).state = { game: "none" } as GameState;
  return parsed;
}

/** Run one command and throw a human message on rejection. */
export function send(raw: string): GameState {
  const r = cmd(raw);
  if (!r.ok) throw new Error(r.error || "Illegal move.");
  return r.state;
}

export const seed = (): number => (Date.now() ^ (Math.random() * 1e9)) >>> 0;
