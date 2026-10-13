// Mirror of the engine's JSON. The engine owns the rules; these types only describe state.
export type CardId = number; // 0..51, suit = id/13, rank = id%13

export interface LastAction {
  kind: string;
  player: number;
  card?: CardId | null;
  cards?: CardId[];
  suit?: number | null;
  meld?: number | null;
  plaza?: number | null;
}

export interface PlayerInfo {
  name: string;
  count: number;
  score: number;
  banners?: number;
  deadwood?: number;
  human: boolean;
  hand: CardId[] | null;
}

export interface BaseState {
  game: string;
  players: PlayerInfo[];
  you: number;
  turn: number;
  phase: string;
  winner: number | null;
  last: LastAction | null;
  log: string[];
}

export interface Crazy8State extends BaseState {
  game: "crazy8";
  top: CardId;
  activeSuit: number;
  drawCount: number;
  discardCount: number;
  legal: number[];
  canDraw: boolean;
  canPass: boolean;
  mustPass: boolean;
  pending: CardId | null;
  roundPoints: number;
}

export interface MeldInfo {
  id: number;
  kind: "trio" | "escalera";
  owner: number;
  cards: CardId[];
}

export interface RunsState extends BaseState {
  game: "runs";
  drawCount: number;
  discard: CardId[];
  melds: MeldInfo[];
  extends: number[][];
  taken: CardId | null;
}

export interface KlPile {
  c: CardId;
  up: boolean;
}
export interface KlondikeState {
  game: "klondike";
  tableau: KlPile[][];
  foundations: CardId[][];
  stockCount: number;
  waste: CardId[];
  wasteCount: number;
  moves: number;
  won: boolean;
  openBoard: boolean;
  canUndo?: boolean;
}

export interface PlazaCard {
  c: CardId;
  o: number;
}
export interface PlazaInfo {
  suit: number;
  cards: PlazaCard[];
  leader: number | null;
}
export interface PlazaState extends BaseState {
  game: "plaza";
  plazas: PlazaInfo[];
  stockCount: number;
  preview: string[][];
  resolution: { plaza: number; winner: number; cards: PlazaCard[] } | null;
  lastPlay: { player: number; card: CardId; plaza: number } | null;
  tie: boolean;
}

export type GameState = Crazy8State | RunsState | KlondikeState | PlazaState | { game: "none" };

export interface Reply {
  ok: boolean;
  error: string | null;
  data: Record<string, unknown> | null;
  state: GameState;
}

export const SUITS = ["♠", "♥", "♦", "♣"] as const;
export const SUIT_NAMES = ["Spades", "Hearts", "Diamonds", "Clubs"] as const;
export const RANKS = ["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"] as const;

export const suitOf = (c: CardId): number => Math.floor(c / 13);
export const rankOf = (c: CardId): number => c % 13;
export const redSuit = (s: number): boolean => s === 1 || s === 2;
export const isRed = (c: CardId): boolean => redSuit(suitOf(c));
export const cardName = (c: CardId): string => `${RANKS[rankOf(c)]}${SUITS[suitOf(c)]}`;
