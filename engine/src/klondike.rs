//! Classic Klondike solitaire, draw one, unlimited redeals.

use crate::cards::*;
use crate::json::{self, Obj};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Loc {
    Tableau(usize),
    Foundation(usize),
    /// `f` with no index: route to the foundation of the card's own suit.
    FoundationAuto,
    Waste,
}

pub fn parse_loc(s: &str) -> Option<Loc> {
    let b = s.as_bytes();
    match b.first()? {
        b'w' => Some(Loc::Waste),
        b't' => s[1..].parse::<usize>().ok().filter(|n| *n < 7).map(Loc::Tableau),
        b'f' => {
            if s.len() == 1 {
                Some(Loc::FoundationAuto)
            } else {
                s[1..].parse::<usize>().ok().filter(|n| *n < 4).map(Loc::Foundation)
            }
        }
        _ => None,
    }
}

pub fn loc_label(l: Loc) -> String {
    match l {
        Loc::Waste => "w".into(),
        Loc::Tableau(t) => format!("t{t}"),
        Loc::Foundation(f) => format!("f{f}"),
        Loc::FoundationAuto => "f".into(),
    }
}

/// Descending ranks in alternating colours.
pub fn valid_sequence(cards: &[Card]) -> bool {
    cards.windows(2).all(|w| {
        w[1].rank() + 1 == w[0].rank() && w[1].is_red() != w[0].is_red()
    })
}

#[derive(Clone)]
pub struct Klondike {
    pub tableau: Vec<Vec<(Card, bool)>>,
    pub foundations: Vec<Vec<Card>>,
    pub stock: Vec<Card>,
    pub waste: Vec<Card>,
    pub moves: u32,
    pub redeals: u32,
    pub won: bool,
    pub last: Option<(Vec<Card>, String)>,
}

impl Klondike {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let mut deck = full_deck();
        rng.shuffle(&mut deck);
        let mut tableau = vec![Vec::new(); 7];
        for col in 0..7usize {
            for row in 0..=col {
                let c = deck.pop().unwrap();
                tableau[col].push((c, row == col));
            }
        }
        Klondike {
            tableau,
            foundations: vec![Vec::new(); 4],
            stock: deck,
            waste: Vec::new(),
            moves: 0,
            redeals: 0,
            won: false,
            last: None,
        }
    }

    pub fn draw(&mut self) -> Result<(), String> {
        if self.stock.is_empty() {
            if self.waste.is_empty() {
                return Err("No cards left to turn.".into());
            }
            self.stock = self.waste.drain(..).rev().collect();
            self.redeals += 1;
            self.last = Some((Vec::new(), "redeal".into()));
            return Ok(());
        }
        let c = self.stock.pop().unwrap();
        self.waste.push(c);
        self.moves += 1;
        self.last = Some((vec![c], "w".into()));
        Ok(())
    }

    fn moving(&self, src: Loc, idx: usize) -> Result<Vec<Card>, String> {
        match src {
            Loc::Waste => self
                .waste
                .last()
                .map(|c| vec![*c])
                .ok_or_else(|| "The waste is empty.".to_string()),
            Loc::Foundation(f) => self.foundations[f]
                .last()
                .map(|c| vec![*c])
                .ok_or_else(|| "That foundation is empty.".to_string()),
            Loc::Tableau(t) => {
                let pile = &self.tableau[t];
                if idx >= pile.len() {
                    return Err("No such card.".into());
                }
                if !pile[idx].1 {
                    return Err("That card is face down.".into());
                }
                let seq: Vec<Card> = pile[idx..].iter().map(|(c, _)| *c).collect();
                if !valid_sequence(&seq) {
                    return Err("Only a descending run in alternating colours travels together.".into());
                }
                Ok(seq)
            }
            Loc::FoundationAuto => Err("Cards cannot be taken from there.".into()),
        }
    }

    fn resolve_dst(&self, dst: Loc, cards: &[Card]) -> Result<Loc, String> {
        match dst {
            Loc::FoundationAuto => Ok(Loc::Foundation(cards[0].suit() as usize)),
            other => Ok(other),
        }
    }

    fn accepts(&self, dst: Loc, cards: &[Card]) -> Result<(), String> {
        match dst {
            Loc::Foundation(f) => {
                if cards.len() != 1 {
                    return Err("Foundations take one card at a time.".into());
                }
                let c = cards[0];
                if c.suit() as usize != f {
                    return Err("Each foundation collects a single suit.".into());
                }
                match self.foundations[f].last() {
                    None => {
                        if c.rank() == ACE {
                            Ok(())
                        } else {
                            Err("A foundation opens with an ace.".into())
                        }
                    }
                    Some(top) => {
                        if top.rank() + 1 == c.rank() {
                            Ok(())
                        } else {
                            Err("Foundations build up in order.".into())
                        }
                    }
                }
            }
            Loc::Tableau(t) => {
                let pile = &self.tableau[t];
                match pile.last() {
                    None => {
                        if cards[0].rank() == KING {
                            Ok(())
                        } else {
                            Err("Only a king may open an empty column.".into())
                        }
                    }
                    Some((top, up)) => {
                        if !*up {
                            return Err("That card is face down.".into());
                        }
                        if top.rank() == cards[0].rank() + 1 && top.is_red() != cards[0].is_red() {
                            Ok(())
                        } else {
                            Err("Columns build down in alternating colours.".into())
                        }
                    }
                }
            }
            _ => Err("Cards cannot go there.".into()),
        }
    }

    pub fn make_move(&mut self, src: Loc, idx: usize, dst: Loc) -> Result<(), String> {
        let cards = self.moving(src, idx)?;
        let dst = self.resolve_dst(dst, &cards)?;
        if src == dst {
            return Err("That card is already there.".into());
        }
        self.accepts(dst, &cards)?;
        // detach
        match src {
            Loc::Waste => {
                self.waste.pop();
            }
            Loc::Foundation(f) => {
                self.foundations[f].pop();
            }
            Loc::Tableau(t) => {
                self.tableau[t].truncate(idx);
                if let Some(last) = self.tableau[t].last_mut() {
                    last.1 = true;
                }
            }
            Loc::FoundationAuto => unreachable!(),
        }
        // attach
        match dst {
            Loc::Foundation(f) => self.foundations[f].push(cards[0]),
            Loc::Tableau(t) => {
                for c in &cards {
                    self.tableau[t].push((*c, true));
                }
            }
            _ => unreachable!(),
        }
        self.moves += 1;
        self.last = Some((cards, loc_label(dst)));
        self.won = self.foundations.iter().all(|f| f.len() == 13);
        Ok(())
    }

    /// Where a given card (and anything riding on it) may legally go.
    pub fn targets(&self, src: Loc, idx: usize) -> Vec<String> {
        let cards = match self.moving(src, idx) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut out = Vec::new();
        for f in 0..4usize {
            let d = Loc::Foundation(f);
            if d != src && self.accepts(d, &cards).is_ok() {
                out.push(loc_label(d));
            }
        }
        for t in 0..7usize {
            let d = Loc::Tableau(t);
            if d != src && self.accepts(d, &cards).is_ok() {
                out.push(loc_label(d));
            }
        }
        out
    }

    /// One-click intent: foundation if possible, otherwise the most useful column.
    pub fn auto(&mut self, src: Loc, idx: usize) -> Result<(), String> {
        let cards = self.moving(src, idx)?;
        let f = Loc::Foundation(cards[0].suit() as usize);
        if cards.len() == 1 && f != src && self.accepts(f, &cards).is_ok() {
            return self.make_move(src, idx, f);
        }
        let mut best: Option<(i32, Loc)> = None;
        for t in 0..7usize {
            let d = Loc::Tableau(t);
            if d == src || self.accepts(d, &cards).is_err() {
                continue;
            }
            let empty = self.tableau[t].is_empty();
            // Prefer a move that frees a face-down card; avoid burning an empty column.
            let mut score = if empty { 0 } else { 10 };
            if let Loc::Tableau(s) = src {
                if idx > 0 && !self.tableau[s][idx - 1].1 {
                    score += 5;
                }
                if idx == 0 && !empty {
                    score += 3;
                }
            }
            if best.as_ref().map(|(b, _)| score > *b).unwrap_or(true) {
                best = Some((score, d));
            }
        }
        match best {
            Some((_, d)) => self.make_move(src, idx, d),
            None => Err("No legal place for that card.".into()),
        }
    }

    /// Sweep every available card home. Used once the board is fully open.
    pub fn autoplay(&mut self) -> Result<(), String> {
        let mut any = false;
        loop {
            let mut moved = false;
            for t in 0..7usize {
                if let Some(len) = Some(self.tableau[t].len()) {
                    if len > 0 {
                        let c = self.tableau[t][len - 1].0;
                        let f = Loc::Foundation(c.suit() as usize);
                        if self.tableau[t][len - 1].1 && self.accepts(f, &[c]).is_ok() {
                            self.make_move(Loc::Tableau(t), len - 1, f)?;
                            moved = true;
                            any = true;
                        }
                    }
                }
            }
            if let Some(c) = self.waste.last().copied() {
                let f = Loc::Foundation(c.suit() as usize);
                if self.accepts(f, &[c]).is_ok() {
                    self.make_move(Loc::Waste, 0, f)?;
                    moved = true;
                    any = true;
                }
            }
            if !moved {
                break;
            }
        }
        if any {
            Ok(())
        } else {
            Err("Nothing can go home yet.".into())
        }
    }

    /// True when every card is visible, so a sweep is guaranteed to finish.
    pub fn open_board(&self) -> bool {
        self.stock.is_empty()
            && self.waste.len() <= 1
            && self.tableau.iter().all(|p| p.iter().all(|(_, up)| *up))
    }

    pub fn to_json(&self) -> String {
        self.write(None)
    }

    pub(crate) fn json_with_undo(&self, can_undo: bool) -> String {
        self.write(Some(can_undo))
    }

    fn write(&self, can_undo: Option<bool>) -> String {
        let tableau = json::arr(&self.tableau, |p| {
            json::arr(p, |(c, up)| {
                Obj::new().num("c", c.0 as i64).flag("up", *up).done()
            })
        });
        let foundations = json::arr(&self.foundations, |f| {
            json::arr(f, |c| c.0.to_string())
        });
        let mut doc = Obj::new()
            .text("game", "klondike")
            .raw("tableau", &tableau)
            .raw("foundations", &foundations)
            .num("stockCount", self.stock.len() as i64)
            .raw(
                "waste",
                &json::arr(
                    &self.waste[self.waste.len().saturating_sub(3)..],
                    |c| c.0.to_string(),
                ),
            )
            .num("wasteCount", self.waste.len() as i64)
            .num("moves", self.moves as i64)
            .flag("won", self.won)
            .flag("openBoard", self.open_board());
        if let Some(v) = can_undo {
            doc = doc.flag("canUndo", v);
        }
        doc.done()
    }
}
