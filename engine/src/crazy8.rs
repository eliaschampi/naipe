//! 8 Locos — Crazy Eights.

use crate::cards::*;
use crate::json::{self, Obj};
use crate::tail;
use crate::NAMES;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Play,
    ChooseSuit,
    Over,
}

#[derive(Clone, Debug)]
pub struct Action {
    pub kind: &'static str,
    pub player: usize,
    pub card: Option<Card>,
    pub suit: Option<u8>,
}

#[derive(Clone)]
pub struct Crazy8 {
    pub hands: Vec<Vec<Card>>,
    pub draw: Vec<Card>,
    pub discard: Vec<Card>,
    pub active_suit: u8,
    pub turn: usize,
    pub phase: Phase,
    pub winner: Option<usize>,
    pub scores: Vec<u32>,
    pub round_points: u32,
    pub drawn_this_turn: bool,
    /// The card just drawn, when it happens to be playable.
    pub pending: Option<Card>,
    pub last: Option<Action>,
    pub log: Vec<String>,
    rng: Rng,
}

impl Crazy8 {
    pub const YOU: usize = crate::YOU;

    pub fn new(seed: u64, players: usize) -> Self {
        let n = players.clamp(2, 4);
        let mut rng = Rng::new(seed);
        let mut deck = full_deck();
        rng.shuffle(&mut deck);
        let per = if n >= 3 { 5 } else { 7 };
        let mut hands = vec![Vec::new(); n];
        for _ in 0..per {
            for h in hands.iter_mut() {
                h.push(deck.pop().unwrap());
            }
        }
        for h in hands.iter_mut() {
            sort_hand(h, true);
        }
        // Reveal the starter. An eight would leave the suit unresolved, so it
        // goes back into the deck and another card is turned.
        let mut top;
        loop {
            top = deck.pop().unwrap();
            if !top.is_eight() {
                break;
            }
            let at = rng.below(deck.len() + 1);
            deck.insert(at, top);
        }
        let mut g = Crazy8 {
            hands,
            draw: deck,
            discard: vec![top],
            active_suit: top.suit(),
            turn: 0,
            phase: Phase::Play,
            winner: None,
            scores: vec![0; n],
            round_points: 0,
            drawn_this_turn: false,
            pending: None,
            last: None,
            log: Vec::new(),
            rng,
        };
        g.log.push(format!("Turn card: {}", pretty(top)));
        g
    }

    pub fn players(&self) -> usize {
        self.hands.len()
    }
    pub fn top(&self) -> Card {
        *self.discard.last().unwrap()
    }

    pub fn is_legal(&self, c: Card) -> bool {
        c.is_eight() || c.suit() == self.active_suit || c.rank() == self.top().rank()
    }

    /// Hand indices the current player may legally play right now.
    pub fn legal_indices(&self, p: usize) -> Vec<usize> {
        if self.phase != Phase::Play || self.turn != p {
            return Vec::new();
        }
        match self.pending {
            // After drawing, only the drawn card may be played.
            Some(c) => self.hands[p]
                .iter()
                .position(|x| *x == c)
                .map(|i| vec![i])
                .unwrap_or_default(),
            None => {
                if self.drawn_this_turn {
                    Vec::new()
                } else {
                    (0..self.hands[p].len())
                        .filter(|i| self.is_legal(self.hands[p][*i]))
                        .collect()
                }
            }
        }
    }

    pub fn can_draw(&self) -> bool {
        self.phase == Phase::Play && !self.drawn_this_turn
    }
    pub fn must_pass(&self) -> bool {
        self.phase == Phase::Play && self.drawn_this_turn && self.pending.is_none()
    }

    // ---- commands -------------------------------------------------------

    pub fn play(&mut self, idx: usize) -> Result<(), String> {
        if self.phase != Phase::Play {
            return Err("Not a moment for playing a card.".into());
        }
        let p = self.turn;
        if idx >= self.hands[p].len() {
            return Err("No such card.".into());
        }
        let c = self.hands[p][idx];
        if let Some(pc) = self.pending {
            if pc != c {
                return Err("After drawing you may only play the card you drew.".into());
            }
        } else if !self.is_legal(c) {
            return Err(format!(
                "{} does not match {} or {}.",
                pretty(c),
                suit_name(self.active_suit),
                rank_name(self.top().rank())
            ));
        }
        self.hands[p].remove(idx);
        self.discard.push(c);
        self.active_suit = c.suit();
        self.pending = None;
        self.last = Some(Action { kind: "play", player: p, card: Some(c), suit: None });
        self.log.push(format!("{} played {}", NAMES[p], pretty(c)));
        if self.hands[p].is_empty() {
            self.finish(p);
            return Ok(());
        }
        if c.is_eight() {
            self.phase = Phase::ChooseSuit;
        } else {
            self.end_turn();
        }
        Ok(())
    }

    pub fn choose_suit(&mut self, suit: u8) -> Result<(), String> {
        if self.phase != Phase::ChooseSuit {
            return Err("No wild suit to choose.".into());
        }
        if suit > 3 {
            return Err("Unknown suit.".into());
        }
        self.active_suit = suit;
        self.phase = Phase::Play;
        let p = self.turn;
        self.last = Some(Action { kind: "suit", player: p, card: None, suit: Some(suit) });
        self.log.push(format!("{} called {}", NAMES[p], suit_name(suit)));
        self.end_turn();
        Ok(())
    }

    pub fn draw(&mut self) -> Result<(), String> {
        if self.phase != Phase::Play {
            return Err("Not a moment for drawing.".into());
        }
        if self.drawn_this_turn {
            return Err("Only one card may be drawn per turn.".into());
        }
        self.replenish();
        self.drawn_this_turn = true;
        let p = self.turn;
        match self.draw.pop() {
            None => {
                self.last = Some(Action { kind: "empty", player: p, card: None, suit: None });
                self.log.push("The stock is exhausted.".into());
                Ok(())
            }
            Some(c) => {
                self.hands[p].push(c);
                sort_hand(&mut self.hands[p], true);
                self.last = Some(Action { kind: "draw", player: p, card: Some(c), suit: None });
                self.log.push(format!("{} drew {}", NAMES[p], pretty(c)));
                // The turn stays open either way. A match may be played or kept;
                // anything else has to be passed, so the player actually sees it.
                if self.is_legal(c) {
                    self.pending = Some(c);
                }
                Ok(())
            }
        }
    }

    pub fn pass(&mut self) -> Result<(), String> {
        if self.phase != Phase::Play {
            return Err("Nothing to pass on.".into());
        }
        if !self.drawn_this_turn {
            return Err("Draw a card before passing.".into());
        }
        let p = self.turn;
        self.last = Some(Action { kind: "pass", player: p, card: None, suit: None });
        self.log.push(format!("{} passed", NAMES[p]));
        self.end_turn();
        Ok(())
    }

    fn end_turn(&mut self) {
        self.drawn_this_turn = false;
        self.pending = None;
        self.turn = (self.turn + 1) % self.players();
    }

    /// Keeps the current top discard, reshuffles everything beneath it.
    fn replenish(&mut self) {
        if self.draw.is_empty() && self.discard.len() > 1 {
            let top = self.discard.pop().unwrap();
            self.draw = std::mem::take(&mut self.discard);
            self.rng.shuffle(&mut self.draw);
            self.discard.push(top);
            self.log.push("Stock rebuilt from the discards.".into());
        }
    }

    fn finish(&mut self, w: usize) {
        self.phase = Phase::Over;
        self.winner = Some(w);
        let pts: u32 = self
            .hands
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != w)
            .map(|(_, h)| h.iter().map(|c| c.value_eights()).sum::<u32>())
            .sum();
        self.round_points = pts;
        self.scores[w] += pts;
        self.log.push(format!("{} wins the round for {} points", NAMES[w], pts));
    }

    // ---- computer players ----------------------------------------------

    fn suit_counts(&self, p: usize) -> [usize; 4] {
        let mut c = [0usize; 4];
        for card in &self.hands[p] {
            c[card.suit() as usize] += 1;
        }
        c
    }

    /// The suit an eight should call: mostly "where my hand lives".
    pub fn ai_pick_suit(&self, p: usize) -> u8 {
        let counts = self.suit_counts(p);
        let mut best = 0u8;
        let mut best_score = f64::MIN;
        for s in 0..4usize {
            let mut score = counts[s] as f64 * 10.0;
            // Prefer a suit where the hand also has good cards to follow with.
            score += self.hands[p]
                .iter()
                .filter(|c| c.suit() as usize == s && !c.is_eight())
                .map(|c| c.value() as f64 * 0.2)
                .sum::<f64>();
            if score > best_score {
                best_score = score;
                best = s as u8;
            }
        }
        best
    }

    fn rate_play(&self, p: usize, idx: usize) -> f64 {
        let c = self.hands[p][idx];
        let hand = &self.hands[p];
        if hand.len() == 1 {
            return 10_000.0; // wins immediately
        }
        if c.is_eight() {
            // Eights are the most valuable cards to hold; only spend them when
            // nothing else is legal (handled by the caller) or the hand is tiny.
            return -100.0 + (5 - hand.len().min(5)) as f64 * 10.0;
        }
        let after_same_suit = hand
            .iter()
            .enumerate()
            .filter(|(i, x)| *i != idx && x.suit() == c.suit())
            .count() as f64;
        let mut score = c.value_eights() as f64 * 0.8; // shed points
        score += after_same_suit * 3.0; // keep a follow-up in the active suit
        // A rank match that steers the suit toward our strength is worth a lot.
        if c.suit() != self.active_suit {
            let counts = self.suit_counts(p);
            score += counts[c.suit() as usize] as f64 * 2.0;
        }
        // Hold onto duplicate ranks: they are future rank-match escapes.
        let same_rank = hand
            .iter()
            .enumerate()
            .filter(|(i, x)| *i != idx && x.rank() == c.rank())
            .count() as f64;
        score -= same_rank * 1.5;
        score
    }

    /// One atomic computer action so the interface can narrate each beat.
    pub fn ai_step(&mut self) -> Result<(), String> {
        let p = self.turn;
        if self.phase == Phase::ChooseSuit {
            let s = self.ai_pick_suit(p);
            return self.choose_suit(s);
        }
        if self.phase == Phase::Over {
            return Err("The round is over.".into());
        }
        let legal = self.legal_indices(p);
        let non_eight: Vec<usize> = legal
            .iter()
            .copied()
            .filter(|i| !self.hands[p][*i].is_eight())
            .collect();
        let pool = if non_eight.is_empty() { legal.clone() } else { non_eight };
        if pool.is_empty() {
            if self.can_draw() {
                return self.draw();
            }
            return self.pass();
        }
        let mut best = pool[0];
        let mut best_score = f64::MIN;
        for i in pool {
            let s = self.rate_play(p, i);
            if s > best_score {
                best_score = s;
                best = i;
            }
        }
        self.play(best)
    }

    // ---- serialization --------------------------------------------------

    pub fn to_json(&self) -> String {
        let over = self.phase == Phase::Over;
        let players = crate::players_json(&self.hands, over, |i, o| {
            o.num("score", self.scores[i] as i64)
        });
        let last = match &self.last {
            None => "null".to_string(),
            Some(a) => {
                let mut o = Obj::new().text("kind", a.kind).num("player", a.player as i64);
                o = match a.card {
                    Some(c) => o.num("card", c.0 as i64),
                    None => o.nil("card"),
                };
                o = match a.suit {
                    Some(s) => o.num("suit", s as i64),
                    None => o.nil("suit"),
                };
                o.done()
            }
        };
        Obj::new()
            .text("game", "crazy8")
            .raw("players", &players)
            .num("you", Self::YOU as i64)
            .num("turn", self.turn as i64)
            .text(
                "phase",
                match self.phase {
                    Phase::Play => "play",
                    Phase::ChooseSuit => "choose_suit",
                    Phase::Over => "over",
                },
            )
            .num("top", self.top().0 as i64)
            .num("activeSuit", self.active_suit as i64)
            .num("drawCount", self.draw.len() as i64)
            .num("discardCount", self.discard.len() as i64)
            .raw("legal", &json::usizes(&self.legal_indices(Self::YOU)))
            .flag("canDraw", self.can_draw() && self.turn == Self::YOU)
            .flag("mustPass", self.must_pass() && self.turn == Self::YOU)
            .flag("canPass", self.turn == Self::YOU && (self.must_pass() || self.pending.is_some()))
            .opt_i64("pending", self.pending.map(|c| c.0 as i64))
            .opt_i64("winner", self.winner.map(|w| w as i64))
            .num("roundPoints", self.round_points as i64)
            .raw("last", &last)
            .raw("log", &json::strings(&tail(&self.log, 6)))
            .done()
    }
}

pub fn rank_name(r: u8) -> &'static str {
    [
        "Aces", "Twos", "Threes", "Fours", "Fives", "Sixes", "Sevens", "Eights", "Nines", "Tens",
        "Jacks", "Queens", "Kings",
    ][r as usize]
}
