//! Naipe — the canonical rules engine for all four games.
//!
//! The boundary to the interface is deliberately tiny: commands arrive as a
//! pipe-delimited ASCII string, every reply is a JSON document containing the
//! full next state. No bindings generator, no serialization framework, no
//! second copy of the rules anywhere else.

pub mod cards;
pub mod crazy8;
pub mod json;
pub mod klondike;
pub mod plaza;
pub mod runs;

use std::collections::VecDeque;

use cards::Card;
use crazy8::Crazy8;
use json::Obj;
use klondike::{parse_loc, Klondike};
use plaza::Plaza;
use runs::Runs;

pub const NAMES: [&str; 4] = ["You", "Nora", "Silas", "Iris"];
/// The human always sits here. Every multiplayer game reads this one seat.
pub const YOU: usize = 0;

pub fn tail(v: &[String], n: usize) -> Vec<String> {
    let start = v.len().saturating_sub(n);
    v[start..].to_vec()
}

/// One player document. Opponent hands stay null until the deal is over.
pub fn players_json(
    hands: &[Vec<Card>],
    reveal_all: bool,
    extra: impl Fn(usize, Obj) -> Obj,
) -> String {
    let seats: Vec<usize> = (0..hands.len()).collect();
    json::arr(&seats, |seat| {
        let i = *seat;
        let base = Obj::new()
            .text("name", NAMES[i])
            .num("count", hands[i].len() as i64)
            .flag("human", i == YOU);
        let with = extra(i, base);
        if reveal_all || i == YOU {
            with.raw("hand", &json::arr(&hands[i], |c| c.0.to_string())).done()
        } else {
            with.nil("hand").done()
        }
    })
}

pub enum Session {
    Empty,
    C8(Crazy8),
    Runs(Runs),
    Kl(Klondike),
    Pz(Plaza),
}

pub struct App {
    pub session: Session,
    /// Undo history. Only solitaire has a meaningful, cheat-free undo.
    history: VecDeque<Klondike>,
}

type Reply = Result<Option<String>, String>;

impl App {
    pub fn new() -> Self {
        App { session: Session::Empty, history: VecDeque::new() }
    }

    pub fn exec(&mut self, cmd: &str) -> String {
        let parts: Vec<&str> = cmd.split('|').collect();
        let result = self.run(&parts);
        let state = self.state_json();
        let o = Obj::new();
        let o = match &result {
            Ok(data) => {
                let o = o.flag("ok", true).nil("error");
                match data {
                    Some(d) => o.raw("data", d),
                    None => o.nil("data"),
                }
            }
            Err(e) => o.flag("ok", false).text("error", e).nil("data"),
        };
        o.raw("state", &state).done()
    }

    fn run(&mut self, p: &[&str]) -> Reply {
        let verb = *p.first().unwrap_or(&"");
        if verb == "new" {
            let game = p.get(1).copied().unwrap_or("");
            let seed: u64 = p.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
            let players: usize = p.get(3).and_then(|s| s.parse().ok()).unwrap_or(3);
            self.history.clear();
            self.session = match game {
                "crazy8" => Session::C8(Crazy8::new(seed, players)),
                "runs" => Session::Runs(Runs::new(seed, players)),
                "klondike" => Session::Kl(Klondike::new(seed)),
                "plaza" => Session::Pz(Plaza::new(seed, players)),
                _ => return Err("Unknown game.".into()),
            };
            return Ok(None);
        }
        if verb == "state" {
            return Ok(None);
        }
        match &mut self.session {
            Session::Empty => Err("No game in progress.".into()),
            Session::C8(g) => Self::run_c8(g, verb, p),
            Session::Runs(g) => Self::run_runs(g, verb, p),
            Session::Pz(g) => Self::run_plaza(g, verb, p),
            Session::Kl(_) => self.run_klondike(verb, p),
        }
    }

    fn arg(p: &[&str], i: usize) -> Result<usize, String> {
        p.get(i)
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| "Malformed command.".to_string())
    }

    fn list(p: &[&str], i: usize) -> Result<Vec<usize>, String> {
        let raw = p.get(i).ok_or("Malformed command.")?;
        raw.split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<usize>().map_err(|_| "Malformed command.".to_string()))
            .collect()
    }

    fn run_c8(g: &mut Crazy8, verb: &str, p: &[&str]) -> Reply {
        let human = g.turn == Crazy8::YOU;
        match verb {
            "ai" => {
                if human {
                    return Err("It is your turn.".into());
                }
                g.ai_step().map(|_| None)
            }
            "play" | "suit" | "draw" | "pass" if !human => Err("It is not your turn.".into()),
            "play" => g.play(Self::arg(p, 1)?).map(|_| None),
            "suit" => g.choose_suit(Self::arg(p, 1)? as u8).map(|_| None),
            "draw" => g.draw().map(|_| None),
            "pass" => g.pass().map(|_| None),
            _ => Err("Unknown command.".into()),
        }
    }

    fn run_runs(g: &mut Runs, verb: &str, p: &[&str]) -> Reply {
        let human = g.turn == Runs::YOU;
        match verb {
            "ai" => {
                if human {
                    return Err("It is your turn.".into());
                }
                g.ai_step().map(|_| None)
            }
            "check" => {
                let idx = Self::list(p, 1)?;
                let hand = &g.hands[Runs::YOU];
                if idx.iter().any(|i| *i >= hand.len()) {
                    return Err("Selection is out of date.".into());
                }
                let cards: Vec<cards::Card> = idx.iter().map(|i| hand[*i]).collect();
                let o = match runs::classify(&cards) {
                    Ok(k) => Obj::new().flag("valid", true).text("kind", k.name()).nil("reason"),
                    Err(e) => Obj::new().flag("valid", false).nil("kind").text("reason", &e),
                };
                Ok(Some(o.done()))
            }
            "sort" => {
                g.sort(p.get(1).copied().unwrap_or("suit") == "suit");
                Ok(None)
            }
            "arrange" => g.arrange(Self::arg(p, 1)?, Self::arg(p, 2)?).map(|_| None),
            "verify" => g.verify().map(|_| None),
            _ if !human => Err("It is not your turn.".into()),
            "drawstock" => g.draw_stock().map(|_| None),
            "drawdiscard" => g.draw_discard().map(|_| None),
            "meld" => g.meld(Self::list(p, 1)?).map(|_| None),
            "extend" => g.extend(Self::arg(p, 1)?, Self::arg(p, 2)?).map(|_| None),
            "discard" => g.discard(Self::arg(p, 1)?).map(|_| None),
            _ => Err("Unknown command.".into()),
        }
    }

    fn run_plaza(g: &mut Plaza, verb: &str, p: &[&str]) -> Reply {
        let human = g.turn == Plaza::YOU;
        match verb {
            "ai" => {
                if human {
                    return Err("It is your turn.".into());
                }
                g.ai_step().map(|_| None)
            }
            "play" => {
                if !human {
                    return Err("It is not your turn.".into());
                }
                g.play(Self::arg(p, 1)?, Self::arg(p, 2)?).map(|_| None)
            }
            _ => Err("Unknown command.".into()),
        }
    }

    fn run_klondike(&mut self, verb: &str, p: &[&str]) -> Reply {
        let snapshot = match &self.session {
            Session::Kl(g) => g.clone(),
            _ => return Err("No game in progress.".into()),
        };
        if verb == "undo" {
            return match self.history.pop_back() {
                Some(prev) => {
                    self.session = Session::Kl(prev);
                    Ok(None)
                }
                None => Err("Nothing to undo.".into()),
            };
        }
        let g = match &mut self.session {
            Session::Kl(g) => g,
            _ => unreachable!(),
        };
        if verb == "targets" {
            let src = parse_loc(p.get(1).copied().unwrap_or("")).ok_or("Unknown pile.")?;
            let idx = Self::arg(p, 2).unwrap_or(0);
            let t = g.targets(src, idx);
            return Ok(Some(Obj::new().raw("targets", &json::strings(&t)).done()));
        }
        let outcome = match verb {
            "draw" => g.draw(),
            "move" => {
                let src = parse_loc(p.get(1).copied().unwrap_or("")).ok_or("Unknown pile.")?;
                let dst = parse_loc(p.get(3).copied().unwrap_or("")).ok_or("Unknown pile.")?;
                g.make_move(src, Self::arg(p, 2).unwrap_or(0), dst)
            }
            "auto" => {
                let src = parse_loc(p.get(1).copied().unwrap_or("")).ok_or("Unknown pile.")?;
                g.auto(src, Self::arg(p, 2).unwrap_or(0))
            }
            "autoall" => g.autoplay(),
            _ => return Err("Unknown command.".into()),
        };
        match outcome {
            Ok(()) => {
                self.history.push_back(snapshot);
                if self.history.len() > 400 {
                    self.history.pop_front();
                }
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    pub fn state_json(&self) -> String {
        match &self.session {
            Session::Empty => Obj::new().text("game", "none").done(),
            Session::C8(g) => g.to_json(),
            Session::Runs(g) => g.to_json(),
            Session::Pz(g) => g.to_json(),
            Session::Kl(g) => g.json_with_undo(!self.history.is_empty()),
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// WebAssembly boundary: two shared buffers, one entry point.
// ---------------------------------------------------------------------------

const CAP: usize = 8192;
static mut IN: [u8; CAP] = [0; CAP];
static mut OUT: Vec<u8> = Vec::new();
static mut APP: Option<App> = None;

fn app() -> &'static mut App {
    unsafe {
        let p = std::ptr::addr_of_mut!(APP);
        if (*p).is_none() {
            *p = Some(App::new());
        }
        (*p).as_mut().unwrap()
    }
}

/// Pointer to the command buffer the host writes UTF-8 into.
#[no_mangle]
pub extern "C" fn input_ptr() -> *mut u8 {
    std::ptr::addr_of_mut!(IN) as *mut u8
}

#[no_mangle]
pub extern "C" fn input_capacity() -> usize {
    CAP
}

/// Runs the command currently in the input buffer; returns the reply length.
#[no_mangle]
pub extern "C" fn dispatch(len: usize) -> usize {
    let n = len.min(CAP);
    let cmd = unsafe {
        let slice = std::slice::from_raw_parts(std::ptr::addr_of!(IN) as *const u8, n);
        String::from_utf8_lossy(slice).into_owned()
    };
    let reply = app().exec(&cmd);
    unsafe {
        let out = &mut *std::ptr::addr_of_mut!(OUT);
        out.clear();
        out.extend_from_slice(reply.as_bytes());
        out.len()
    }
}

#[no_mangle]
pub extern "C" fn output_ptr() -> *const u8 {
    unsafe { (*std::ptr::addr_of_mut!(OUT)).as_ptr() }
}

#[cfg(test)]
mod tests;
