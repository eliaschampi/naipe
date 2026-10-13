//! Escaleras y Tríos — runs and sets.

use crate::cards::*;
use crate::json::{self, Obj};
use crate::tail;
use crate::NAMES;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Set,
    Run,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Set => "trio",
            Kind::Run => "escalera",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Draw,
    Build,
    Over,
}

#[derive(Clone, Debug)]
pub struct Meld {
    pub id: usize,
    pub kind: Kind,
    pub cards: Vec<Card>,
    pub owner: usize,
}

#[derive(Clone, Debug)]
pub struct Action {
    pub kind: &'static str,
    pub player: usize,
    pub cards: Vec<Card>,
    pub meld: Option<usize>,
}

/// Ascending values of a same-suit consecutive sequence, if the cards form one.
pub fn run_values(cards: &[Card]) -> Option<Vec<u8>> {
    if cards.len() < 2 {
        return None;
    }
    let suit = cards[0].suit();
    if cards.iter().any(|c| c.suit() != suit) {
        return None;
    }
    for ace_high in [false, true] {
        let mut v: Vec<u8> = cards
            .iter()
            .map(|c| if ace_high { c.high() } else { c.low() })
            .collect();
        v.sort_unstable();
        if v.windows(2).all(|w| w[1] == w[0] + 1) {
            return Some(v);
        }
    }
    None
}

fn order_run(cards: &mut Vec<Card>) -> bool {
    for ace_high in [false, true] {
        let mut v = cards.clone();
        v.sort_by_key(|c| if ace_high { c.high() } else { c.low() });
        let vals: Vec<u8> = v
            .iter()
            .map(|c| if ace_high { c.high() } else { c.low() })
            .collect();
        if vals.windows(2).all(|w| w[1] == w[0] + 1) {
            *cards = v;
            return true;
        }
    }
    false
}

/// The single source of truth for what a legal combination is.
pub fn classify(cards: &[Card]) -> Result<Kind, String> {
    if cards.len() < 3 {
        return Err("A combination needs at least three cards.".into());
    }
    let mut seen = cards.to_vec();
    seen.sort_unstable();
    if seen.windows(2).any(|w| w[0] == w[1]) {
        return Err("The same card cannot be used twice.".into());
    }
    let same_rank = cards.iter().all(|c| c.rank() == cards[0].rank());
    if same_rank {
        if cards.len() > 4 {
            return Err("A trio holds three or four cards, no more.".into());
        }
        let mut suits: Vec<u8> = cards.iter().map(|c| c.suit()).collect();
        suits.sort_unstable();
        if suits.windows(2).any(|w| w[0] == w[1]) {
            return Err("A trio needs a different suit for every card.".into());
        }
        return Ok(Kind::Set);
    }
    let same_suit = cards.iter().all(|c| c.suit() == cards[0].suit());
    if same_suit {
        return match run_values(cards) {
            Some(_) => Ok(Kind::Run),
            None => Err("An escalera must run in unbroken sequence — and an ace sits at one end only, never bridging K-A-2.".into()),
        };
    }
    Err("Match one rank for a trio, or one suit in sequence for an escalera.".into())
}

pub fn can_extend(meld: &Meld, c: Card) -> bool {
    match meld.kind {
        Kind::Set => {
            meld.cards.len() < 4
                && c.rank() == meld.cards[0].rank()
                && !meld.cards.iter().any(|x| x.suit() == c.suit())
        }
        Kind::Run => {
            if c.suit() != meld.cards[0].suit() {
                return false;
            }
            let mut t = meld.cards.clone();
            t.push(c);
            run_values(&t).is_some()
        }
    }
}

#[derive(Clone)]
pub struct Runs {
    pub hands: Vec<Vec<Card>>,
    pub draw: Vec<Card>,
    pub discard: Vec<Card>,
    pub melds: Vec<Meld>,
    next_id: usize,
    pub turn: usize,
    pub phase: Phase,
    pub taken: Option<Card>,
    pub winner: Option<usize>,
    pub scores: Vec<u32>,
    pub last: Option<Action>,
    pub log: Vec<String>,
    rng: Rng,
}

impl Runs {
    pub const YOU: usize = crate::YOU;
    pub const HAND: usize = 10;

    pub fn new(seed: u64, players: usize) -> Self {
        let n = players.clamp(2, 4);
        let mut rng = Rng::new(seed);
        let mut deck = full_deck();
        rng.shuffle(&mut deck);
        let mut hands = vec![Vec::new(); n];
        for _ in 0..Self::HAND {
            for h in hands.iter_mut() {
                h.push(deck.pop().unwrap());
            }
        }
        for h in hands.iter_mut() {
            sort_hand(h, true);
        }
        let first = deck.pop().unwrap();
        Runs {
            hands,
            draw: deck,
            discard: vec![first],
            melds: Vec::new(),
            next_id: 0,
            turn: 0,
            phase: Phase::Draw,
            taken: None,
            winner: None,
            scores: vec![0; n],
            last: None,
            log: vec![format!("Discard opens with {}", pretty(first))],
            rng,
        }
    }

    pub fn players(&self) -> usize {
        self.hands.len()
    }

    // ---- commands -------------------------------------------------------

    pub fn draw_stock(&mut self) -> Result<(), String> {
        if self.phase != Phase::Draw {
            return Err("You have already drawn.".into());
        }
        self.replenish();
        let p = self.turn;
        match self.draw.pop() {
            None => {
                self.stalemate();
                Ok(())
            }
            Some(c) => {
                self.hands[p].push(c);
                self.phase = Phase::Build;
                self.taken = None;
                self.last = Some(Action { kind: "draw_stock", player: p, cards: vec![c], meld: None });
                self.log.push(format!("{} drew from the stock", NAMES[p]));
                Ok(())
            }
        }
    }

    pub fn draw_discard(&mut self) -> Result<(), String> {
        if self.phase != Phase::Draw {
            return Err("You have already drawn.".into());
        }
        let p = self.turn;
        let c = self.discard.pop().ok_or("The discard pile is empty.")?;
        self.hands[p].push(c);
        self.phase = Phase::Build;
        self.taken = Some(c);
        self.last = Some(Action { kind: "draw_discard", player: p, cards: vec![c], meld: None });
        self.log.push(format!("{} took {} from the discards", NAMES[p], pretty(c)));
        Ok(())
    }

    pub fn meld(&mut self, mut idx: Vec<usize>) -> Result<(), String> {
        if self.phase != Phase::Build {
            return Err("Draw a card first.".into());
        }
        let p = self.turn;
        idx.sort_unstable();
        idx.dedup();
        if idx.iter().any(|i| *i >= self.hands[p].len()) {
            return Err("Selection is out of date.".into());
        }
        let cards: Vec<Card> = idx.iter().map(|i| self.hands[p][*i]).collect();
        let kind = classify(&cards)?;
        for i in idx.iter().rev() {
            self.hands[p].remove(*i);
        }
        let mut cards = cards;
        match kind {
            Kind::Run => {
                order_run(&mut cards);
            }
            Kind::Set => cards.sort_by_key(|c| c.suit()),
        }
        let id = self.next_id;
        self.next_id += 1;
        self.melds.push(Meld { id, kind, cards: cards.clone(), owner: p });
        self.last = Some(Action { kind: "meld", player: p, cards, meld: Some(id) });
        self.log.push(format!("{} laid down an {}", NAMES[p], kind.name()));
        self.check_out(p);
        Ok(())
    }

    pub fn extend(&mut self, meld_id: usize, hand_idx: usize) -> Result<(), String> {
        if self.phase != Phase::Build {
            return Err("Draw a card first.".into());
        }
        let p = self.turn;
        if hand_idx >= self.hands[p].len() {
            return Err("Selection is out of date.".into());
        }
        let c = self.hands[p][hand_idx];
        let pos = self
            .melds
            .iter()
            .position(|m| m.id == meld_id)
            .ok_or("No such combination on the table.")?;
        if !can_extend(&self.melds[pos], c) {
            return Err(format!("{} does not fit that combination.", pretty(c)));
        }
        self.hands[p].remove(hand_idx);
        let m = &mut self.melds[pos];
        m.cards.push(c);
        match m.kind {
            Kind::Run => {
                order_run(&mut m.cards);
            }
            Kind::Set => m.cards.sort_by_key(|c| c.suit()),
        }
        self.last = Some(Action { kind: "extend", player: p, cards: vec![c], meld: Some(meld_id) });
        self.log.push(format!("{} extended with {}", NAMES[p], pretty(c)));
        self.check_out(p);
        Ok(())
    }

    pub fn discard(&mut self, idx: usize) -> Result<(), String> {
        if self.phase != Phase::Build {
            return Err("Draw a card first.".into());
        }
        let p = self.turn;
        if idx >= self.hands[p].len() {
            return Err("No such card.".into());
        }
        let c = self.hands[p][idx];
        if self.hands[p].len() > 1 && self.taken == Some(c) {
            return Err("The card you took from the discards must stay this turn.".into());
        }
        self.hands[p].remove(idx);
        self.discard.push(c);
        self.last = Some(Action { kind: "discard", player: p, cards: vec![c], meld: None });
        self.log.push(format!("{} discarded {}", NAMES[p], pretty(c)));
        if self.hands[p].is_empty() {
            self.finish(p);
        } else {
            self.taken = None;
            self.phase = Phase::Draw;
            self.turn = (self.turn + 1) % self.players();
        }
        Ok(())
    }

    pub fn arrange(&mut self, from: usize, to: usize) -> Result<(), String> {
        let h = &mut self.hands[Self::YOU];
        if from >= h.len() || to >= h.len() {
            return Err("Selection is out of date.".into());
        }
        let c = h.remove(from);
        h.insert(to, c);
        Ok(())
    }

    pub fn sort(&mut self, by_suit: bool) {
        sort_hand(&mut self.hands[Self::YOU], by_suit);
    }

    fn check_out(&mut self, p: usize) {
        if self.hands[p].is_empty() {
            self.finish(p);
        }
    }

    fn replenish(&mut self) {
        if self.draw.is_empty() && self.discard.len() > 1 {
            let top = self.discard.pop().unwrap();
            self.draw = std::mem::take(&mut self.discard);
            self.rng.shuffle(&mut self.draw);
            self.discard.push(top);
            self.log.push("Stock rebuilt from the discards.".into());
        }
    }

    fn hand_value(&self, p: usize) -> u32 {
        self.hands[p].iter().map(|c| c.value()).sum()
    }

    fn finish(&mut self, w: usize) {
        // Never trust the caller: re-verify every combination on the table.
        for m in &self.melds {
            debug_assert!(classify(&m.cards).is_ok(), "invalid meld on table");
        }
        self.phase = Phase::Over;
        self.winner = Some(w);
        let pts: u32 = (0..self.players())
            .filter(|i| *i != w)
            .map(|i| self.hand_value(i))
            .sum();
        self.scores[w] += pts;
        self.log
            .push(format!("{} goes out and takes {} points", NAMES[w], pts));
    }

    fn stalemate(&mut self) {
        let mut best = 0usize;
        for i in 1..self.players() {
            if self.hand_value(i) < self.hand_value(best) {
                best = i;
            }
        }
        self.phase = Phase::Over;
        self.winner = Some(best);
        self.log
            .push(format!("Cards run out — {} holds the lightest hand", NAMES[best]));
    }

    /// Every card must be accounted for exactly once, and every meld legal.
    pub fn verify(&self) -> Result<(), String> {
        let mut seen = vec![0u8; 52];
        for h in &self.hands {
            for c in h {
                seen[c.0 as usize] += 1;
            }
        }
        for c in self.draw.iter().chain(self.discard.iter()) {
            seen[c.0 as usize] += 1;
        }
        for m in &self.melds {
            classify(&m.cards)?;
            for c in &m.cards {
                seen[c.0 as usize] += 1;
            }
        }
        if seen.iter().any(|n| *n != 1) {
            return Err("card conservation violated".into());
        }
        Ok(())
    }

    // ---- computer players ----------------------------------------------

    fn candidates(hand: &[Card]) -> Vec<Vec<usize>> {
        let mut out: Vec<Vec<usize>> = Vec::new();
        for s in 0..4u8 {
            let mut idx: Vec<usize> = (0..hand.len()).filter(|i| hand[*i].suit() == s).collect();
            idx.sort_by_key(|i| hand[*i].low());
            let mut i = 0;
            while i < idx.len() {
                let mut j = i + 1;
                while j < idx.len() && hand[idx[j]].low() == hand[idx[j - 1]].low() + 1 {
                    j += 1;
                }
                if j - i >= 3 {
                    out.push(idx[i..j].to_vec());
                }
                i = j;
            }
            let at = |r: u8| idx.iter().find(|i| hand[**i].rank() == r).copied();
            if let (Some(q), Some(k), Some(a)) = (at(QUEEN), at(KING), at(ACE)) {
                let mut v = vec![q, k, a];
                let mut r = JACK as i8;
                while r >= 1 {
                    match at(r as u8) {
                        Some(x) => {
                            v.insert(0, x);
                            r -= 1;
                        }
                        None => break,
                    }
                }
                out.push(v);
            }
        }
        for r in 0..13u8 {
            let g: Vec<usize> = (0..hand.len()).filter(|i| hand[*i].rank() == r).collect();
            if g.len() >= 3 {
                out.push(g);
            }
        }
        out.sort_by_key(|v| std::cmp::Reverse(v.len()));
        out
    }

    fn greedy(hand: &[Card]) -> Vec<Vec<usize>> {
        let mut used = vec![false; hand.len()];
        let mut chosen = Vec::new();
        for c in Self::candidates(hand) {
            if c.iter().all(|i| !used[*i]) {
                for i in &c {
                    used[*i] = true;
                }
                chosen.push(c);
            }
        }
        chosen
    }

    fn melded_count(hand: &[Card]) -> usize {
        Self::greedy(hand).iter().map(|m| m.len()).sum()
    }

    /// How much a card is worth keeping: rank pairs and suited neighbours.
    fn keep_score(hand: &[Card], i: usize) -> f64 {
        let c = hand[i];
        let mut s = 0.0;
        for (j, o) in hand.iter().enumerate() {
            if j == i {
                continue;
            }
            if o.rank() == c.rank() {
                s += 4.0;
            } else if o.suit() == c.suit() {
                let d = (o.low() as i32 - c.low() as i32).abs();
                if d == 1 {
                    s += 3.0;
                } else if d == 2 {
                    s += 1.5;
                }
            }
        }
        s
    }

    pub fn ai_step(&mut self) -> Result<(), String> {
        let p = self.turn;
        match self.phase {
            Phase::Over => Err("The round is over.".into()),
            Phase::Draw => {
                let mut take = false;
                if let Some(top) = self.discard.last().copied() {
                    let before = Self::melded_count(&self.hands[p]);
                    let mut trial = self.hands[p].clone();
                    trial.push(top);
                    if Self::melded_count(&trial) > before {
                        take = true;
                    }
                    if !take && self.melds.iter().any(|m| can_extend(m, top)) {
                        take = true;
                    }
                }
                if take {
                    self.draw_discard()
                } else {
                    self.draw_stock()
                }
            }
            Phase::Build => {
                let hand = self.hands[p].clone();
                let melds = Self::greedy(&hand);
                if let Some(m) = melds.first() {
                    return self.meld(m.clone());
                }
                // Lay off cards that are doing nothing for us.
                let protect: Vec<usize> = melds.iter().flatten().copied().collect();
                let mut best: Option<(usize, usize)> = None;
                for i in 0..hand.len() {
                    if protect.contains(&i) {
                        continue;
                    }
                    if hand.len() > 2 && Self::keep_score(&hand, i) >= 4.0 {
                        continue;
                    }
                    for m in &self.melds {
                        if can_extend(m, hand[i]) {
                            best = Some((m.id, i));
                            break;
                        }
                    }
                    if best.is_some() {
                        break;
                    }
                }
                if let Some((mid, i)) = best {
                    return self.extend(mid, i);
                }
                let mut pick = 0usize;
                let mut worst = f64::MAX;
                for i in 0..hand.len() {
                    if hand.len() > 1 && self.taken == Some(hand[i]) {
                        continue;
                    }
                    let s = Self::keep_score(&hand, i) - hand[i].value() as f64 * 0.12;
                    if s < worst {
                        worst = s;
                        pick = i;
                    }
                }
                self.discard(pick)
            }
        }
    }

    // ---- serialization --------------------------------------------------

    pub fn extendable(&self, p: usize) -> Vec<Vec<usize>> {
        self.melds
            .iter()
            .map(|m| {
                if self.phase != Phase::Build || self.turn != p {
                    return Vec::new();
                }
                (0..self.hands[p].len())
                    .filter(|i| can_extend(m, self.hands[p][*i]))
                    .collect()
            })
            .collect()
    }

    pub fn to_json(&self) -> String {
        let over = self.phase == Phase::Over;
        let players = crate::players_json(&self.hands, over, |i, o| {
            o.num("score", self.scores[i] as i64)
                .num("deadwood", self.hand_value(i) as i64)
        });
        let melds = json::arr(&self.melds, |m| {
            Obj::new()
                .num("id", m.id as i64)
                .text("kind", m.kind.name())
                .num("owner", m.owner as i64)
                .raw("cards", &json::arr(&m.cards, |c| c.0.to_string()))
                .done()
        });
        let extends = json::arr(&self.extendable(Self::YOU), |v| json::usizes(v));
        let last = match &self.last {
            None => "null".to_string(),
            Some(a) => {
                let o = Obj::new()
                    .text("kind", a.kind)
                    .num("player", a.player as i64)
                    .raw("cards", &json::arr(&a.cards, |c| c.0.to_string()));
                match a.meld {
                    Some(m) => o.num("meld", m as i64),
                    None => o.nil("meld"),
                }
                .done()
            }
        };
        Obj::new()
            .text("game", "runs")
            .raw("players", &players)
            .num("you", Self::YOU as i64)
            .num("turn", self.turn as i64)
            .text(
                "phase",
                match self.phase {
                    Phase::Draw => "draw",
                    Phase::Build => "build",
                    Phase::Over => "over",
                },
            )
            .num("drawCount", self.draw.len() as i64)
            .raw("discard", &json::arr(&self.discard, |c| c.0.to_string()))
            .raw("melds", &melds)
            .raw("extends", &extends)
            .opt_i64("taken", self.taken.map(|c| c.0 as i64))
            .opt_i64("winner", self.winner.map(|w| w as i64))
            .raw("last", &last)
            .raw("log", &json::strings(&tail(&self.log, 6)))
            .done()
    }
}
