use crate::cards::*;
use crate::crazy8::{Crazy8, Phase as C8Phase};
use crate::klondike::{parse_loc, valid_sequence, Klondike, Loc};
use crate::plaza::{power, resolve, Plaza};
use crate::runs::{can_extend, classify, Kind, Meld, Phase as RPhase, Runs};
use crate::App;

fn c(s: &str) -> Card {
    Card::from_label(s).unwrap_or_else(|| panic!("bad card {s}"))
}
fn cs(list: &[&str]) -> Vec<Card> {
    list.iter().map(|s| c(s)).collect()
}

// ---------------------------------------------------------------- cards ----

#[test]
fn card_encoding_round_trips() {
    for id in 0..52u8 {
        let card = Card(id);
        assert_eq!(Card::from_label(&card.label()), Some(card));
    }
    assert_eq!(c("AS").suit(), 0);
    assert_eq!(c("AS").rank(), ACE);
    assert!(c("QH").is_red());
    assert!(!c("QC").is_red());
    assert!(c("8D").is_eight());
    assert_eq!(c("10C").value(), 10);
    assert_eq!(c("KH").value(), 10);
    assert_eq!(c("AS").value(), 1);
    assert_eq!(c("8S").value_eights(), 50);
    assert_eq!(c("AS").low(), 1);
    assert_eq!(c("AS").high(), 14);
}

#[test]
fn shuffle_is_a_permutation_and_seed_stable() {
    let mut a = full_deck();
    let mut b = full_deck();
    Rng::new(7).shuffle(&mut a);
    Rng::new(7).shuffle(&mut b);
    assert_eq!(a, b, "same seed must reproduce the deal");
    let mut sorted = a.clone();
    sorted.sort();
    assert_eq!(sorted, full_deck());
    let mut d = full_deck();
    Rng::new(8).shuffle(&mut d);
    assert_ne!(a, d);
}

// ------------------------------------------------------------- crazy 8 ----

fn c8(hand: &[&str], top: &str) -> Crazy8 {
    let mut g = Crazy8::new(42, 3);
    g.hands[0] = cs(hand);
    g.discard = vec![c(top)];
    g.active_suit = c(top).suit();
    g.turn = 0;
    g.phase = C8Phase::Play;
    g.drawn_this_turn = false;
    g.pending = None;
    g
}

#[test]
fn c8_deal_sizes_and_starter() {
    let g = Crazy8::new(3, 3);
    assert_eq!(g.players(), 3);
    for h in &g.hands {
        assert_eq!(h.len(), 5);
    }
    let two = Crazy8::new(3, 2);
    for h in &two.hands {
        assert_eq!(h.len(), 7);
    }
    // Every seed must open on a non-eight.
    for seed in 0..300u64 {
        let g = Crazy8::new(seed, 3);
        assert!(!g.top().is_eight(), "seed {seed} opened on an eight");
        let mut all: Vec<Card> = g.draw.clone();
        all.extend(g.discard.iter().copied());
        for h in &g.hands {
            all.extend(h.iter().copied());
        }
        all.sort();
        assert_eq!(all, full_deck(), "seed {seed} lost a card");
    }
}

#[test]
fn c8_matches_by_suit_and_rank() {
    let g = c8(&["2H", "7C", "KD"], "7H");
    assert!(g.is_legal(c("2H")), "suit match");
    assert!(g.is_legal(c("7C")), "rank match");
    assert!(!g.is_legal(c("KD")), "neither suit nor rank");
    assert_eq!(g.legal_indices(0), vec![0, 1]);
}

#[test]
fn c8_rejects_illegal_card() {
    let mut g = c8(&["KD"], "7H");
    let err = g.play(0).unwrap_err();
    assert!(err.contains("does not match"), "{err}");
    assert_eq!(g.hands[0].len(), 1);
    assert_eq!(g.top(), c("7H"));
}

#[test]
fn c8_eight_is_wild_and_sets_the_suit() {
    let mut g = c8(&["8S", "2C", "3C"], "7H");
    assert!(g.is_legal(c("8S")));
    g.play(0).unwrap();
    assert_eq!(g.phase, C8Phase::ChooseSuit);
    assert_eq!(g.turn, 0, "the turn waits for the suit call");
    g.choose_suit(3).unwrap();
    assert_eq!(g.active_suit, 3);
    assert_eq!(g.phase, C8Phase::Play);
    assert_eq!(g.turn, 1);
}

#[test]
fn c8_chosen_suit_governs_the_next_play() {
    let mut g = c8(&["8S"], "7H");
    g.hands[0] = cs(&["8S", "2D"]);
    g.play(0).unwrap();
    g.choose_suit(2).unwrap(); // diamonds
    g.turn = 0;
    g.phase = C8Phase::Play;
    assert!(g.is_legal(c("9D")), "chosen suit is live");
    assert!(!g.is_legal(c("9C")), "old suit is dead");
    assert!(g.is_legal(c("8C")), "another eight is always legal");
    assert!(!g.is_legal(c("9S")), "the eight's printed suit is irrelevant");
}

#[test]
fn c8_choose_suit_is_rejected_out_of_phase() {
    let mut g = c8(&["2H"], "7H");
    assert!(g.choose_suit(1).is_err());
}

#[test]
fn c8_draw_then_play_or_pass() {
    let mut g = c8(&["KD"], "7H");
    g.draw = vec![c("2H")];
    g.draw().unwrap();
    assert_eq!(g.pending, Some(c("2H")));
    assert_eq!(g.turn, 0, "a playable draw keeps the turn open");
    assert_eq!(g.legal_indices(0).len(), 1, "only the drawn card may be played");
    assert!(g.draw().is_err(), "one draw per turn");
    let i = g.legal_indices(0)[0];
    g.play(i).unwrap();
    assert_eq!(g.top(), c("2H"));
    assert_eq!(g.turn, 1);
}

#[test]
fn c8_unplayable_draw_waits_for_a_pass() {
    let mut g = c8(&["KD"], "7H");
    g.draw = vec![c("4C")];
    g.draw().unwrap();
    assert_eq!(g.turn, 0, "the player must see the card before the turn ends");
    assert!(g.pending.is_none());
    assert!(g.must_pass());
    let view = g.to_json();
    assert!(view.contains("\"mustPass\":true"), "{view}");
    assert!(view.contains("\"canPass\":true"), "{view}");
    assert!(g.legal_indices(0).is_empty());
    g.pass().unwrap();
    assert_eq!(g.turn, 1);
    assert!(g.hands[0].contains(&c("4C")));
}

#[test]
fn c8_playable_draw_can_be_held() {
    let mut g = c8(&["KD"], "7H");
    g.draw = vec![c("2H")];
    g.draw().unwrap();
    assert_eq!(g.pending, Some(c("2H")));
    g.pass().unwrap();
    assert_eq!(g.turn, 1);
    assert!(g.hands[0].contains(&c("2H")), "passing keeps the matching draw");
}

#[test]
fn c8_pass_requires_a_draw() {
    let mut g = c8(&["KD"], "7H");
    assert!(g.pass().is_err());
    g.draw = vec![];
    g.discard = vec![c("7H")];
    g.draw().unwrap(); // stock empty: nothing to take
    assert!(g.must_pass());
    g.pass().unwrap();
    assert_eq!(g.turn, 1);
}

#[test]
fn c8_rebuilds_the_stock_from_the_discards() {
    let mut g = c8(&["KD"], "7H");
    g.draw = vec![];
    g.discard = cs(&["2C", "5D", "9S", "7H"]);
    g.draw().unwrap();
    assert_eq!(g.discard.len(), 1, "the live top card stays put");
    assert_eq!(g.discard[0], c("7H"));
    assert_eq!(g.draw.len(), 2, "three recycled, one drawn");
    assert_eq!(g.hands[0].len(), 2);
}

#[test]
fn c8_victory_and_scoring() {
    let mut g = c8(&["2H"], "7H");
    g.hands[1] = cs(&["8S", "KD"]); // 50 + 10
    g.hands[2] = cs(&["AC", "3D"]); // 1 + 3
    g.play(0).unwrap();
    assert_eq!(g.phase, C8Phase::Over);
    assert_eq!(g.winner, Some(0));
    assert_eq!(g.round_points, 64);
    assert_eq!(g.scores[0], 64);
}

#[test]
fn c8_winning_with_an_eight_needs_no_suit_call() {
    let mut g = c8(&["8S"], "7H");
    g.play(0).unwrap();
    assert_eq!(g.phase, C8Phase::Over);
    assert_eq!(g.winner, Some(0));
}

#[test]
fn c8_ai_prefers_ordinary_cards_and_calls_its_best_suit() {
    let mut g = c8(&[], "7H");
    g.turn = 1;
    g.hands[1] = cs(&["8C", "2H"]);
    g.ai_step().unwrap();
    assert_eq!(g.top(), c("2H"), "the eight is worth keeping");

    let mut g = c8(&[], "7H");
    g.turn = 1;
    g.hands[1] = cs(&["8C", "KS", "QS", "3S"]);
    g.ai_step().unwrap(); // nothing ordinary is legal, so the eight goes down
    assert_eq!(g.top(), c("8C"));
    assert_eq!(g.phase, C8Phase::ChooseSuit);
    g.ai_step().unwrap();
    assert_eq!(g.active_suit, 0, "it calls the suit it holds");
}

#[test]
fn c8_ai_never_stalls() {
    for seed in 0..60u64 {
        let mut g = Crazy8::new(seed, 4);
        let mut steps = 0;
        while g.phase != C8Phase::Over && steps < 4000 {
            g.ai_step().unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            steps += 1;
        }
        assert!(g.phase == C8Phase::Over, "seed {seed} never finished");
        assert!(g.hands[g.winner.unwrap()].is_empty());
    }
}

#[test]
fn c8_hides_opponent_hands() {
    let g = Crazy8::new(5, 3);
    let s = g.to_json();
    assert!(s.contains("\"hand\":null"), "opponents must stay hidden");
    assert_eq!(s.matches("\"hand\":null").count(), 2);
}

// ------------------------------------------------------- runs and sets ----

#[test]
fn set_of_three_is_valid() {
    assert_eq!(classify(&cs(&["7C", "7D", "7H"])), Ok(Kind::Set));
}

#[test]
fn set_of_four_is_valid() {
    assert_eq!(classify(&cs(&["QS", "QH", "QD", "QC"])), Ok(Kind::Set));
}

#[test]
fn set_needs_three_cards() {
    assert!(classify(&cs(&["7C", "7D"])).is_err());
}

#[test]
fn set_rejects_repeated_cards() {
    let e = classify(&cs(&["7C", "7C", "7D"])).unwrap_err();
    assert!(e.contains("twice"), "{e}");
}

#[test]
fn mixed_rubbish_is_rejected() {
    let e = classify(&cs(&["7C", "8D", "QH"])).unwrap_err();
    assert!(e.contains("trio") || e.contains("escalera"), "{e}");
}

#[test]
fn run_same_suit_is_valid() {
    assert_eq!(classify(&cs(&["4H", "5H", "6H"])), Ok(Kind::Run));
    assert_eq!(
        classify(&cs(&["9C", "10C", "JC", "QC", "KC"])),
        Ok(Kind::Run)
    );
}

#[test]
fn run_rejects_mixed_suits() {
    assert!(classify(&cs(&["4H", "5S", "6H"])).is_err());
}

#[test]
fn run_rejects_gaps() {
    let e = classify(&cs(&["4H", "6H", "7H"])).unwrap_err();
    assert!(e.contains("sequence"), "{e}");
}

#[test]
fn ace_runs_low() {
    assert_eq!(classify(&cs(&["AS", "2S", "3S"])), Ok(Kind::Run));
}

#[test]
fn ace_runs_high() {
    assert_eq!(classify(&cs(&["QD", "KD", "AD"])), Ok(Kind::Run));
}

#[test]
fn ace_never_wraps_around() {
    assert!(classify(&cs(&["KH", "AH", "2H"])).is_err());
    assert!(classify(&cs(&["QH", "KH", "AH", "2H"])).is_err());
}

#[test]
fn extending_melds() {
    let set = Meld { id: 0, kind: Kind::Set, cards: cs(&["7C", "7D", "7H"]), owner: 0 };
    assert!(can_extend(&set, c("7S")));
    assert!(!can_extend(&set, c("8S")));
    let full = Meld { id: 1, kind: Kind::Set, cards: cs(&["7C", "7D", "7H", "7S"]), owner: 0 };
    assert!(!can_extend(&full, c("8S")), "a trio caps at four");

    let run = Meld { id: 2, kind: Kind::Run, cards: cs(&["4H", "5H", "6H"]), owner: 0 };
    assert!(can_extend(&run, c("3H")), "extend downward");
    assert!(can_extend(&run, c("7H")), "extend upward");
    assert!(!can_extend(&run, c("8H")), "no jumping a gap");
    assert!(!can_extend(&run, c("3S")), "suit must match");

    let low = Meld { id: 3, kind: Kind::Run, cards: cs(&["AS", "2S", "3S"]), owner: 0 };
    assert!(can_extend(&low, c("4S")));
    assert!(!can_extend(&low, c("KS")), "the ace is already low");

    let high = Meld { id: 4, kind: Kind::Run, cards: cs(&["JD", "QD", "KD"]), owner: 0 };
    assert!(can_extend(&high, c("AD")), "the ace caps a run");
    assert!(can_extend(&high, c("10D")));
}

#[test]
fn runs_turn_order_is_enforced() {
    let mut g = Runs::new(11, 3);
    assert_eq!(g.phase, RPhase::Draw);
    assert!(g.discard(0).is_err(), "must draw first");
    assert!(g.meld(vec![0, 1, 2]).is_err(), "must draw first");
    g.draw_stock().unwrap();
    assert_eq!(g.phase, RPhase::Build);
    assert_eq!(g.hands[0].len(), 11);
    assert!(g.draw_stock().is_err(), "one draw per turn");
    g.discard(0).unwrap();
    assert_eq!(g.turn, 1);
    assert_eq!(g.hands[0].len(), 10);
    g.verify().unwrap();
}

#[test]
fn runs_cannot_re_discard_the_taken_card() {
    let mut g = Runs::new(12, 3);
    let top = *g.discard.last().unwrap();
    g.draw_discard().unwrap();
    let i = g.hands[0].iter().position(|x| *x == top).unwrap();
    let e = g.discard(i).unwrap_err();
    assert!(e.contains("must stay"), "{e}");
    let other = (0..g.hands[0].len()).find(|j| *j != i).unwrap();
    g.discard(other).unwrap();
}

#[test]
fn runs_meld_and_win_by_going_out() {
    let mut g = Runs::new(13, 3);
    g.hands[0] = cs(&["4H", "5H", "6H", "9C"]);
    g.hands[1] = cs(&["KS", "AD"]); // 10 + 1
    g.hands[2] = cs(&["5C"]); // 5
    g.draw = vec![c("7H")];
    g.turn = 0;
    g.phase = RPhase::Draw;
    g.draw_stock().unwrap();
    g.meld(vec![0, 1, 2, 4]).unwrap(); // 4-5-6-7 of hearts
    assert_eq!(g.melds.len(), 1);
    assert_eq!(g.melds[0].cards, cs(&["4H", "5H", "6H", "7H"]));
    assert_eq!(g.hands[0], cs(&["9C"]));
    g.discard(0).unwrap();
    assert_eq!(g.phase, RPhase::Over);
    assert_eq!(g.winner, Some(0));
    assert_eq!(g.scores[0], 16);
}

#[test]
fn runs_stalemate_keeps_the_hand_and_scores_nothing() {
    let mut g = Runs::new(3, 3);
    g.hands[0] = cs(&["KS"]);
    g.hands[1] = cs(&["AS"]);
    g.hands[2] = cs(&["5C"]);
    g.draw.clear();
    g.discard = cs(&["2D"]);
    g.melds.clear();
    g.turn = 0;
    g.phase = RPhase::Draw;
    g.draw_stock().unwrap();
    assert_eq!(g.phase, RPhase::Over);
    assert_eq!(g.winner, Some(1), "Nora holds the only ace");
    assert_eq!(g.scores, vec![0, 0, 0]);
    assert_eq!(g.hands[1], cs(&["AS"]));
    let json = g.to_json();
    assert!(json.contains("\"winner\":1"), "{json}");
    assert!(
        json.contains("\"name\":\"Nora\",\"count\":1,\"human\":false,\"score\":0,\"deadwood\":1"),
        "{json}"
    );
    assert!(
        g.log.iter().any(|l| l == "Cards run out — Nora holds the lightest hand"),
        "{:?}",
        g.log
    );
}

#[test]
fn runs_going_out_by_melting_every_card() {
    let mut g = Runs::new(14, 3);
    g.hands[0] = cs(&["7C", "7D", "7H"]);
    g.turn = 0;
    g.phase = RPhase::Build;
    g.meld(vec![0, 1, 2]).unwrap();
    assert_eq!(g.phase, RPhase::Over);
    assert_eq!(g.winner, Some(0));
}

#[test]
fn runs_extend_command_moves_the_card_to_the_table() {
    let mut g = Runs::new(15, 3);
    g.melds.clear();
    g.hands[0] = cs(&["7S", "2D", "9H"]);
    g.turn = 0;
    g.phase = RPhase::Build;
    g.melds.push(Meld { id: 0, kind: Kind::Set, cards: cs(&["7C", "7D", "7H"]), owner: 1 });
    let ext = g.extendable(0);
    assert_eq!(ext[0], vec![0], "only the seven fits");
    assert!(g.extend(0, 1).is_err());
    g.extend(0, 0).unwrap();
    assert_eq!(g.melds[0].cards.len(), 4);
    assert_eq!(g.hands[0], cs(&["2D", "9H"]));
}

#[test]
fn runs_ai_finishes_and_keeps_the_table_legal() {
    for seed in 0..40u64 {
        let mut g = Runs::new(seed, 3);
        let mut steps = 0;
        while g.phase != RPhase::Over && steps < 6000 {
            g.ai_step().unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            g.verify().unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            steps += 1;
        }
        assert_eq!(g.phase, RPhase::Over, "seed {seed} never finished");
        for m in &g.melds {
            classify(&m.cards).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        }
    }
}

#[test]
fn runs_hides_opponent_hands() {
    let g = Runs::new(16, 3);
    assert_eq!(g.to_json().matches("\"hand\":null").count(), 2);
}

// ------------------------------------------------------------ klondike ----

#[test]
fn klondike_deal_is_a_proper_pyramid() {
    let g = Klondike::new(21);
    let mut all = Vec::new();
    for (i, col) in g.tableau.iter().enumerate() {
        assert_eq!(col.len(), i + 1);
        for (j, (card, up)) in col.iter().enumerate() {
            assert_eq!(*up, j == i, "only the last card of a column shows");
            all.push(*card);
        }
    }
    all.extend(g.stock.iter().copied());
    all.sort();
    assert_eq!(all, full_deck());
    assert_eq!(g.stock.len(), 24);
}

#[test]
fn klondike_sequence_rule() {
    assert!(valid_sequence(&cs(&["10H", "9S", "8D"])));
    assert!(!valid_sequence(&cs(&["10H", "9D"])), "same colour");
    assert!(!valid_sequence(&cs(&["10H", "8S"])), "gap");
    assert!(valid_sequence(&cs(&["KS"])));
}

fn bare() -> Klondike {
    let mut g = Klondike::new(1);
    for col in g.tableau.iter_mut() {
        col.clear();
    }
    for f in g.foundations.iter_mut() {
        f.clear();
    }
    g.stock.clear();
    g.waste.clear();
    g
}

#[test]
fn klondike_tableau_needs_alternating_colours_descending() {
    let mut g = bare();
    g.tableau[0] = vec![(c("10H"), true)];
    g.waste = vec![c("9S")];
    g.make_move(Loc::Waste, 0, Loc::Tableau(0)).unwrap();
    assert_eq!(g.tableau[0].len(), 2);

    let mut g = bare();
    g.tableau[0] = vec![(c("10H"), true)];
    g.waste = vec![c("9D")];
    assert!(g.make_move(Loc::Waste, 0, Loc::Tableau(0)).is_err(), "same colour");

    let mut g = bare();
    g.tableau[0] = vec![(c("10H"), true)];
    g.waste = vec![c("8S")];
    assert!(g.make_move(Loc::Waste, 0, Loc::Tableau(0)).is_err(), "not descending");
}

#[test]
fn klondike_only_a_king_opens_an_empty_column() {
    let mut g = bare();
    g.waste = vec![c("QS")];
    let e = g.make_move(Loc::Waste, 0, Loc::Tableau(3)).unwrap_err();
    assert!(e.contains("king"), "{e}");
    g.waste = vec![c("KS")];
    g.make_move(Loc::Waste, 0, Loc::Tableau(3)).unwrap();
    assert_eq!(g.tableau[3].len(), 1);
}

#[test]
fn klondike_king_headed_run_may_move_to_an_empty_column() {
    let mut g = bare();
    g.tableau[0] = vec![(c("5C"), false), (c("KS"), true), (c("QH"), true), (c("JC"), true)];
    g.make_move(Loc::Tableau(0), 1, Loc::Tableau(4)).unwrap();
    assert_eq!(g.tableau[4].len(), 3);
    assert_eq!(g.tableau[0].len(), 1);
    assert!(g.tableau[0][0].1, "the uncovered card flips face up");
}

#[test]
fn klondike_broken_run_cannot_travel_together() {
    let mut g = bare();
    g.tableau[0] = vec![(c("KS"), true), (c("QH"), true), (c("5C"), true)];
    let e = g.make_move(Loc::Tableau(0), 0, Loc::Tableau(4)).unwrap_err();
    assert!(e.contains("alternating"), "{e}");
}

#[test]
fn klondike_face_down_cards_cannot_be_taken() {
    let mut g = bare();
    g.tableau[0] = vec![(c("KS"), false)];
    assert!(g.make_move(Loc::Tableau(0), 0, Loc::Tableau(4)).is_err());
}

#[test]
fn klondike_foundations_sequence_and_match_suit() {
    let mut g = bare();
    g.waste = vec![c("2S")];
    assert!(g.make_move(Loc::Waste, 0, Loc::FoundationAuto).is_err(), "needs an ace first");
    g.waste = vec![c("AS")];
    g.make_move(Loc::Waste, 0, Loc::FoundationAuto).unwrap();
    assert_eq!(g.foundations[0], cs(&["AS"]));
    g.waste = vec![c("2H")];
    assert!(g.make_move(Loc::Waste, 0, Loc::Foundation(0)).is_err(), "wrong suit");
    g.waste = vec![c("3S")];
    assert!(g.make_move(Loc::Waste, 0, Loc::Foundation(0)).is_err(), "out of order");
    g.waste = vec![c("2S")];
    g.make_move(Loc::Waste, 0, Loc::Foundation(0)).unwrap();
    assert_eq!(g.foundations[0].len(), 2);
}

#[test]
fn klondike_foundation_takes_one_card_only() {
    let mut g = bare();
    g.foundations[0] = cs(&["AS"]);
    g.tableau[0] = vec![(c("3H"), true), (c("2S"), true)];
    assert!(g.make_move(Loc::Tableau(0), 0, Loc::FoundationAuto).is_err());
    g.make_move(Loc::Tableau(0), 1, Loc::Foundation(0)).unwrap();
    assert_eq!(g.foundations[0].len(), 2);
}

#[test]
fn klondike_stock_waste_and_redeal() {
    let mut g = bare();
    g.stock = cs(&["3C", "2C", "AC"]); // AC is on top
    g.draw().unwrap();
    assert_eq!(g.waste, cs(&["AC"]));
    g.draw().unwrap();
    g.draw().unwrap();
    assert_eq!(g.waste, cs(&["AC", "2C", "3C"]));
    assert!(g.stock.is_empty());
    g.draw().unwrap(); // redeal
    assert!(g.waste.is_empty());
    assert_eq!(g.stock, cs(&["3C", "2C", "AC"]), "order is preserved through a redeal");
    assert_eq!(g.redeals, 1);
    g.waste.clear();
    g.stock.clear();
    assert!(g.draw().is_err(), "nothing left to turn");
}

#[test]
fn klondike_targets_and_auto() {
    let mut g = bare();
    g.foundations[0] = cs(&["AS"]);
    g.tableau[1] = vec![(c("3H"), true)];
    g.tableau[2] = vec![(c("2S"), true)];
    let t = g.targets(Loc::Tableau(2), 0);
    assert!(t.contains(&"f0".to_string()));
    assert!(t.contains(&"t1".to_string()));
    g.auto(Loc::Tableau(2), 0).unwrap();
    assert_eq!(g.foundations[0].len(), 2, "the foundation wins a one-click move");
}

#[test]
fn klondike_auto_prefers_freeing_a_hidden_card() {
    let mut g = bare();
    g.tableau[0] = vec![(c("4C"), false), (c("9S"), true)];
    g.tableau[1] = vec![(c("10H"), true)];
    g.tableau[2] = vec![(c("10D"), true)];
    g.auto(Loc::Tableau(0), 1).unwrap();
    assert_eq!(g.tableau[0].len(), 1);
    assert!(g.tableau[0][0].1);
}

#[test]
fn klondike_victory_detection() {
    let mut g = bare();
    for s in 0..4u8 {
        g.foundations[s as usize] = (0..13).map(|r| Card(s * 13 + r)).collect();
    }
    g.foundations[3].pop();
    g.waste = vec![Card(3 * 13 + 12)];
    assert!(!g.won);
    g.make_move(Loc::Waste, 0, Loc::FoundationAuto).unwrap();
    assert!(g.won);
}

#[test]
fn klondike_autoplay_sweeps_an_open_board() {
    let mut g = bare();
    g.tableau[0] = vec![(c("AS"), true)];
    g.tableau[1] = vec![(c("AH"), true)];
    g.tableau[2] = vec![(c("2S"), true)];
    assert!(g.open_board());
    g.autoplay().unwrap();
    assert_eq!(g.foundations[0].len(), 2);
    assert_eq!(g.foundations[1].len(), 1);
    assert!(g.autoplay().is_err(), "nothing further to sweep");
}

#[test]
fn klondike_undo_restores_the_previous_position() {
    let mut app = App::new();
    app.exec("new|klondike|99|1");
    let before = app.state_json();
    let reply = app.exec("draw");
    assert!(reply.contains("\"ok\":true"));
    assert_ne!(app.state_json(), before);
    let reply = app.exec("undo");
    assert!(reply.contains("\"ok\":true"));
    assert_eq!(app.state_json(), before, "undo must be exact");
    assert!(app.exec("undo").contains("\"ok\":false"));
}

#[test]
fn klondike_locations_parse() {
    assert_eq!(parse_loc("t0"), Some(Loc::Tableau(0)));
    assert_eq!(parse_loc("t6"), Some(Loc::Tableau(6)));
    assert_eq!(parse_loc("t7"), None);
    assert_eq!(parse_loc("f"), Some(Loc::FoundationAuto));
    assert_eq!(parse_loc("f3"), Some(Loc::Foundation(3)));
    assert_eq!(parse_loc("f4"), None);
    assert_eq!(parse_loc("w"), Some(Loc::Waste));
    assert_eq!(parse_loc("x"), None);
}

// --------------------------------------------------------------- plaza ----

#[test]
fn plaza_suited_cards_outrank_foreigners() {
    // Plaza of spades (suit 0).
    assert!(power(c("2S"), 0) > power(c("AH"), 0), "any spade beats any outsider");
    assert!(power(c("AS"), 0) > power(c("KS"), 0));
    assert!(power(c("AH"), 0) > power(c("KD"), 0));
    assert_eq!(power(c("QH"), 0), power(c("QD"), 0), "foreign suits are equal");
}

#[test]
fn plaza_resolution_and_tie_break() {
    let cards = vec![(c("QH"), 1), (c("KD"), 2), (c("3S"), 0)];
    assert_eq!(resolve(&cards, 0), 2, "the humble spade takes the spade plaza");
    let tie = vec![(c("QH"), 1), (c("QD"), 2)];
    assert_eq!(resolve(&tie, 0), 0, "the earlier card holds a tie");
}

#[test]
fn plaza_third_card_closes_and_awards_a_banner() {
    let mut g = Plaza::new(31, 3);
    g.hands[0] = cs(&["3S"]);
    g.hands[1] = cs(&["QH"]);
    g.hands[2] = cs(&["KD"]);
    g.stock.clear();
    g.turn = 1;
    g.play(0, 0).unwrap(); // Nora: QH into spades
    assert_eq!(g.leader(0), Some(1));
    g.play(0, 0).unwrap(); // Silas: KD into spades
    assert_eq!(g.leader(0), Some(2));
    g.play(0, 0).unwrap(); // You: 3S closes it
    assert_eq!(g.banners[0], 1);
    assert_eq!(g.plazas[0].len(), 0, "the plaza reopens empty");
    let r = g.last_resolution.as_ref().unwrap();
    assert_eq!(r.winner, 0);
    assert_eq!(r.cards.len(), 3);
}

#[test]
fn plaza_rejects_full_plazas_and_bad_indices() {
    let mut g = Plaza::new(32, 3);
    g.plazas[1] = vec![(c("2H"), 1), (c("3H"), 2)];
    g.hands[0] = cs(&["4C", "5C"]);
    assert!(g.play(9, 1).is_err(), "no such card");
    assert!(g.play(0, 4).is_err(), "no such plaza");
    g.plazas[1].push((c("4H"), 1));
    assert!(g.play(0, 1).is_err(), "plaza is full");
}

#[test]
fn plaza_preview_labels_every_option() {
    let mut g = Plaza::new(33, 3);
    g.hands[0] = cs(&["3S", "2H"]);
    g.plazas[0] = vec![(c("QH"), 1), (c("KD"), 2)];
    let p = g.preview(0);
    assert_eq!(p[0][0], "win", "3S closes the spade plaza in our favour");
    assert_eq!(p[1][0], "gift", "2H closes it for somebody else");
    assert_eq!(p[0][1], "lead", "an empty plaza is ours by default");
}

#[test]
fn plaza_plays_out_completely_and_conserves_cards() {
    for seed in 0..80u64 {
        for players in 2..=4usize {
            let mut g = Plaza::new(seed, players);
            let mut steps = 0;
            while g.phase != crate::plaza::Phase::Over && steps < 200 {
                g.ai_step().unwrap_or_else(|e| panic!("seed {seed}: {e}"));
                assert!(g.plazas.iter().all(|p| p.len() < 3), "a plaza was left closed");
                steps += 1;
            }
            assert_eq!(g.phase, crate::plaza::Phase::Over, "seed {seed} never finished");
            assert!(g.hands.iter().all(|h| h.is_empty()));
            assert!(g.stock.is_empty());
            let on_table: usize = g.plazas.iter().map(|p| p.len()).sum();
            let claimed: u32 = g.banners.iter().sum();
            assert_eq!(on_table + claimed as usize * 3, 52, "cards went missing");
        }
    }
}

#[test]
fn plaza_ai_takes_a_banner_it_can_win() {
    let mut g = Plaza::new(34, 3);
    g.turn = 1;
    g.hands[1] = cs(&["2S", "KC"]);
    g.plazas[0] = vec![(c("QH"), 0), (c("JD"), 2)];
    g.stock.clear();
    g.ai_step().unwrap();
    assert_eq!(g.banners[1], 1, "a cheap spade closes it for free");
}

#[test]
fn plaza_hides_opponent_hands() {
    let g = Plaza::new(35, 3);
    assert_eq!(g.to_json().matches("\"hand\":null").count(), 2);
}

// ------------------------------------------------------ command surface ----

#[test]
fn unknown_commands_are_rejected_cleanly() {
    let mut app = App::new();
    assert!(app.exec("state").contains("\"game\":\"none\""));
    assert!(app.exec("draw").contains("No game in progress"));
    assert!(app.exec("new|nope|1|3").contains("Unknown game"));
    app.exec("new|crazy8|1|3");
    assert!(app.exec("frobnicate").contains("Unknown command"));
    assert!(app.exec("play|99").contains("\"ok\":false"));
    assert!(app.exec("state").contains("\"ok\":true"));
}

#[test]
fn human_commands_are_refused_out_of_turn() {
    let mut app = App::new();
    app.exec("new|plaza|4|3");
    assert!(app.exec("ai").contains("It is your turn"));
    app.exec("play|0|0");
    assert!(app.exec("play|0|0").contains("It is not your turn"));
    assert!(app.exec("ai").contains("\"ok\":true"));
}

#[test]
fn every_reply_is_shaped_the_same() {
    let mut app = App::new();
    for cmd in ["new|runs|8|3", "check|0,1,2", "sort|rank", "drawstock", "verify"] {
        let r = app.exec(cmd);
        assert!(r.starts_with('{') && r.ends_with('}'), "{cmd} -> {r}");
        assert!(r.contains("\"state\":{"), "{cmd} -> {r}");
    }
}
