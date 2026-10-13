//! Cards, decks and deterministic shuffling.
//!
//! A card is a single byte, `0..52`.
//!   suit = id / 13   -> 0 spades, 1 hearts, 2 diamonds, 3 clubs
//!   rank = id % 13   -> 0 ace, 1 two, ... 7 eight, ... 10 jack, 11 queen, 12 king

/// Rank index of an Eight (ace is 0).
pub const EIGHT: u8 = 7;
pub const ACE: u8 = 0;
pub const KING: u8 = 12;
pub const QUEEN: u8 = 11;
pub const JACK: u8 = 10;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Card(pub u8);

impl Card {
    #[inline]
    pub fn suit(self) -> u8 {
        self.0 / 13
    }
    #[inline]
    pub fn rank(self) -> u8 {
        self.0 % 13
    }
    #[inline]
    pub fn is_red(self) -> bool {
        matches!(self.suit(), 1 | 2)
    }
    #[inline]
    pub fn is_eight(self) -> bool {
        self.rank() == EIGHT
    }
    /// Ace counted low: A=1 .. K=13.
    #[inline]
    pub fn low(self) -> u8 {
        self.rank() + 1
    }
    /// Ace counted high: 2=2 .. K=13, A=14.
    #[inline]
    pub fn high(self) -> u8 {
        if self.rank() == ACE {
            14
        } else {
            self.rank() + 1
        }
    }
    /// Standard pip value: ace 1, pips face value, court cards 10.
    pub fn value(self) -> u32 {
        match self.rank() {
            ACE => 1,
            r if r >= JACK => 10,
            r => (r + 1) as u32,
        }
    }
    /// Crazy Eights value: eights are worth 50.
    pub fn value_eights(self) -> u32 {
        if self.is_eight() {
            50
        } else {
            self.value()
        }
    }
    pub fn label(self) -> String {
        format!(
            "{}{}",
            RANK_GLYPH[self.rank() as usize],
            SUIT_LETTER[self.suit() as usize]
        )
    }
    pub fn from_label(s: &str) -> Option<Card> {
        let b = s.as_bytes();
        if b.len() < 2 {
            return None;
        }
        let (rs, ss) = s.split_at(s.len() - 1);
        let rank = match rs {
            "A" => 0,
            "2" => 1,
            "3" => 2,
            "4" => 3,
            "5" => 4,
            "6" => 5,
            "7" => 6,
            "8" => 7,
            "9" => 8,
            "10" | "T" => 9,
            "J" => 10,
            "Q" => 11,
            "K" => 12,
            _ => return None,
        };
        let suit = match ss {
            "S" => 0,
            "H" => 1,
            "D" => 2,
            "C" => 3,
            _ => return None,
        };
        Some(Card(suit * 13 + rank))
    }
}

const RANK_GLYPH: [&str; 13] = [
    "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K",
];
const SUIT_LETTER: [&str; 4] = ["S", "H", "D", "C"];
const SUIT_GLYPH: [&str; 4] = ["\u{2660}", "\u{2665}", "\u{2666}", "\u{2663}"];
const SUIT_WORD: [&str; 4] = ["Spades", "Hearts", "Diamonds", "Clubs"];

pub fn suit_name(s: u8) -> &'static str {
    SUIT_WORD[s as usize]
}

/// Log form, with the suit glyph. `label` stays ASCII for commands and tests.
pub fn pretty(c: Card) -> String {
    format!(
        "{}{}",
        RANK_GLYPH[c.rank() as usize],
        SUIT_GLYPH[c.suit() as usize]
    )
}

pub fn full_deck() -> Vec<Card> {
    (0..52).map(Card).collect()
}

/// xorshift64 — small, fast, and reproducible across platforms.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n <= 1 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        let mut i = v.len();
        while i > 1 {
            i -= 1;
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
}

/// Sorts a hand for display: by suit then rank, or by rank then suit.
pub fn sort_hand(hand: &mut [Card], by_suit: bool) {
    if by_suit {
        hand.sort_by_key(|c| (c.suit(), c.rank()));
    } else {
        hand.sort_by_key(|c| (c.rank(), c.suit()));
    }
}
