//! Plaza — an original game.
//!
//! Four plazas, one per suit. Each holds three cards. Play one card into any
//! open plaza; the third card closes it at once. A card of the plaza's own suit
//! outranks every foreigner; otherwise the highest rank takes it, and a tie
//! goes to whoever committed first. The plaza empties and reopens. Most banners
//! when the cards run out wins.
//!
//! The whole game lives in the third seat: closing a plaza you cannot win
//! hands the banner to somebody else, yet every turn you *must* place a card.

use crate::cards::*;
use crate::json::{self, Obj};
use crate::tail;
use crate::NAMES;

pub const HAND: usize = 6;
pub const CAPACITY: usize = 3;

/// A card's standing inside a given plaza. Suited cards live in a higher band.
pub fn power(c: Card, suit: u8) -> u32 {
    if c.suit() == suit {
        14 + c.high() as u32
    } else {
        c.high() as u32
    }
}

/// Index of the winning card. Highest power; earliest placement breaks ties.
pub fn resolve(cards: &[(Card, usize)], suit: u8) -> usize {
    let mut best = 0usize;
    let mut best_p = 0u32;
    for (i, (c, _)) in cards.iter().enumerate() {
        let p = power(*c, suit);
        if p > best_p {
            best_p = p;
            best = i;
        }
    }
    best
}

#[derive(Clone, Debug)]
pub struct Resolution {
    pub plaza: usize,
    pub winner: usize,
    pub cards: Vec<(Card, usize)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Play,
    Over,
}

#[derive(Clone)]
pub struct Plaza {
    pub hands: Vec<Vec<Card>>,
    pub stock: Vec<Card>,
    pub plazas: Vec<Vec<(Card, usize)>>,
    pub banners: Vec<u32>,
    pub turn: usize,
    pub phase: Phase,
    pub winner: Option<usize>,
    pub tie: bool,
    pub last_resolution: Option<Resolution>,
    pub last_play: Option<(usize, Card, usize)>,
    pub log: Vec<String>,
}

impl Plaza {
    pub const YOU: usize = crate::YOU;

    pub fn new(seed: u64, players: usize) -> Self {
        let n = players.clamp(2, 4);
        let mut rng = Rng::new(seed);
        let mut deck = full_deck();
        rng.shuffle(&mut deck);
        let mut hands = vec![Vec::new(); n];
        for _ in 0..HAND {
            for h in hands.iter_mut() {
                h.push(deck.pop().unwrap());
            }
        }
        for h in hands.iter_mut() {
            sort_hand(h, true);
        }
        Plaza {
            hands,
            stock: deck,
            plazas: vec![Vec::new(); 4],
            banners: vec![0; n],
            turn: 0,
            phase: Phase::Play,
            winner: None,
            tie: false,
            last_resolution: None,
            last_play: None,
            log: Vec::new(),
        }
    }

    pub fn players(&self) -> usize {
        self.hands.len()
    }

    /// Who currently holds a plaza, if anybody has placed there.
    pub fn leader(&self, pi: usize) -> Option<usize> {
        if self.plazas[pi].is_empty() {
            None
        } else {
            Some(self.plazas[pi][resolve(&self.plazas[pi], pi as u8)].1)
        }
    }

    pub fn play(&mut self, hand_idx: usize, pi: usize) -> Result<(), String> {
        if self.phase != Phase::Play {
            return Err("The game is over.".into());
        }
        let p = self.turn;
        if pi >= 4 {
            return Err("No such plaza.".into());
        }
        if hand_idx >= self.hands[p].len() {
            return Err("No such card.".into());
        }
        if self.plazas[pi].len() >= CAPACITY {
            return Err("That plaza is full.".into());
        }
        let c = self.hands[p].remove(hand_idx);
        self.plazas[pi].push((c, p));
        self.last_play = Some((p, c, pi));
        self.last_resolution = None;
        self.log
            .push(format!("{} placed {} in {}", NAMES[p], pretty(c), suit_name(pi as u8)));
        if self.plazas[pi].len() == CAPACITY {
            let cards = std::mem::take(&mut self.plazas[pi]);
            let w = cards[resolve(&cards, pi as u8)].1;
            self.banners[w] += 1;
            self.log.push(format!(
                "{} takes the {} banner",
                NAMES[w],
                suit_name(pi as u8)
            ));
            self.last_resolution = Some(Resolution { plaza: pi, winner: w, cards });
        }
        if let Some(c) = self.stock.pop() {
            self.hands[p].push(c);
            sort_hand(&mut self.hands[p], true);
        }
        self.advance();
        Ok(())
    }

    fn advance(&mut self) {
        let n = self.players();
        for k in 1..=n {
            let q = (self.turn + k) % n;
            if !self.hands[q].is_empty() {
                self.turn = q;
                return;
            }
        }
        self.finish();
    }

    fn finish(&mut self) {
        self.phase = Phase::Over;
        let best = *self.banners.iter().max().unwrap();
        let leaders: Vec<usize> = (0..self.players())
            .filter(|i| self.banners[*i] == best)
            .collect();
        self.tie = leaders.len() > 1;
        self.winner = Some(leaders[0]);
        if self.tie {
            self.log.push("The plazas end level.".into());
        } else {
            self.log
                .push(format!("{} wins with {} banners", NAMES[leaders[0]], best));
        }
    }

    /// What each hand card would do in each plaza — the whole tutorial, inline.
    pub fn preview(&self, p: usize) -> Vec<Vec<&'static str>> {
        self.hands[p]
            .iter()
            .map(|c| {
                (0..4)
                    .map(|pi| {
                        if self.plazas[pi].len() >= CAPACITY {
                            return "closed";
                        }
                        let mut cards = self.plazas[pi].clone();
                        cards.push((*c, p));
                        let w = cards[resolve(&cards, pi as u8)].1;
                        if cards.len() == CAPACITY {
                            if w == p {
                                "win"
                            } else {
                                "gift"
                            }
                        } else if w == p {
                            "lead"
                        } else {
                            "trail"
                        }
                    })
                    .collect()
            })
            .collect()
    }

    // ---- computer player ------------------------------------------------

    fn rate(&self, p: usize, ci: usize, pi: usize) -> f64 {
        let c = self.hands[p][ci];
        let suit = pi as u8;
        let mut cards = self.plazas[pi].clone();
        cards.push((c, p));
        let pw = power(c, suit) as f64;
        let cost = c.high() as f64 / 14.0;
        if cards.len() == CAPACITY {
            let w = resolve(&cards, suit);
            if cards[w].1 == p {
                let runner = cards
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != w)
                    .map(|(_, x)| power(x.0, suit))
                    .max()
                    .unwrap_or(0) as f64;
                14.0 - (pw - runner) * 0.12 - cost * 1.5
            } else {
                -10.0 - cost * 2.0
            }
        } else {
            let w = resolve(&cards, suit);
            if cards[w].1 == p {
                let mut s = if cards.len() == CAPACITY - 1 { 7.0 } else { 3.0 };
                s += (pw / 28.0) * 3.0;
                s - cost * 2.5
            } else {
                let mut s = -2.0;
                if cards.len() == CAPACITY - 1 {
                    s -= 3.5;
                }
                s - cost * 2.5
            }
        }
    }

    pub fn ai_step(&mut self) -> Result<(), String> {
        if self.phase == Phase::Over {
            return Err("The game is over.".into());
        }
        let p = self.turn;
        let mut best = (0usize, 0usize);
        let mut best_score = f64::MIN;
        for ci in 0..self.hands[p].len() {
            for pi in 0..4usize {
                if self.plazas[pi].len() >= CAPACITY {
                    continue;
                }
                let s = self.rate(p, ci, pi);
                if s > best_score {
                    best_score = s;
                    best = (ci, pi);
                }
            }
        }
        self.play(best.0, best.1)
    }

    // ---- serialization --------------------------------------------------

    pub fn to_json(&self) -> String {
        let over = self.phase == Phase::Over;
        let players = crate::players_json(&self.hands, over, |i, o| {
            o.num("banners", self.banners[i] as i64)
        });
        let plazas = json::arr(&(0..4).collect::<Vec<_>>(), |pi| {
            let pi = *pi;
            Obj::new()
                .num("suit", pi as i64)
                .raw(
                    "cards",
                    &json::arr(&self.plazas[pi], |(c, o)| {
                        Obj::new().num("c", c.0 as i64).num("o", *o as i64).done()
                    }),
                )
                .opt_i64("leader", self.leader(pi).map(|l| l as i64))
                .done()
        });
        let resolution = match &self.last_resolution {
            None => "null".to_string(),
            Some(r) => Obj::new()
                .num("plaza", r.plaza as i64)
                .num("winner", r.winner as i64)
                .raw(
                    "cards",
                    &json::arr(&r.cards, |(c, o)| {
                        Obj::new().num("c", c.0 as i64).num("o", *o as i64).done()
                    }),
                )
                .done(),
        };
        let last_play = match self.last_play {
            None => "null".to_string(),
            Some((p, c, pi)) => Obj::new()
                .num("player", p as i64)
                .num("card", c.0 as i64)
                .num("plaza", pi as i64)
                .done(),
        };
        // The table only reads this on the human's turn, so opponent beats skip it.
        let preview = if !over && self.turn == Self::YOU {
            json::arr(&self.preview(Self::YOU), |row| json::arr(row, |s| json::quote(s)))
        } else {
            "[]".to_string()
        };
        Obj::new()
            .text("game", "plaza")
            .raw("players", &players)
            .num("you", Self::YOU as i64)
            .num("turn", self.turn as i64)
            .text("phase", if over { "over" } else { "play" })
            .raw("plazas", &plazas)
            .num("stockCount", self.stock.len() as i64)
            .raw("preview", &preview)
            .raw("resolution", &resolution)
            .raw("lastPlay", &last_play)
            .flag("tie", self.tie)
            .opt_i64("winner", self.winner.map(|w| w as i64))
            .raw("log", &json::strings(&tail(&self.log, 6)))
            .done()
    }
}
