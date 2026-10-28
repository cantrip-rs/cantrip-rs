//! The engine for its users (the scenario runner, the benchmarks, later the host and the Python bindings): the rules,
//! and the card table every card crate contributes to, built once per process and never copied.

use std::sync::OnceLock;

pub mod pay;
pub mod undo;
pub mod visibility;

pub use cantrip_api::*;
pub use cantrip_rules::{Game, Rejected, Setup, activation_costs, eligible_companions, revealed_companion, ApplyContract, apply_contract_check, alternative_cast_total, cast_costs, cast_face_of, cast_total, exile_cast_texts, granted_cast_cost, life_for_cast, chars, check_answer, attack_targets, describe, distribution_problem, first_choices, selection_verdict, live, name_problem, nameable, default_name, summoning_sick, granted_cast_total, special_action_costs, sacrifices_for_reduction, replaced_mana_yields, options_with_mana_to_spare, payment_mana_options_with_mana_to_spare};
pub use cantrip_rules::tree;

/// The simplest legal distribution of looked-at cards (the C# `CardDistribution.Greedy`): each destination's
/// minimum first, in order, then each takes what it accepts up to its maximum, cards in the order looked at.
pub fn greedy_distribution(cards: &[ObjectId], destinations: &[Destination]) -> Vec<Vec<ObjectId>> {
    let mut left: Vec<ObjectId> = cards.to_vec();
    let mut placements: Vec<Vec<ObjectId>> = vec![Vec::new(); destinations.len()];
    for pass in 0..2 {
        for (i, d) in destinations.iter().enumerate() {
            let up_to = if pass == 0 { d.min as usize } else if d.max == u8::MAX { usize::MAX } else { d.max as usize };
            let mut j = 0;
            while j < left.len() && placements[i].len() < up_to {
                if d.eligible.contains(&left[j]) {
                    placements[i].push(left.remove(j));
                } else {
                    j += 1;
                }
            }
        }
    }
    placements
}
pub use cantrip_state::{Frame, FrameKind, Mark, Rng};

/// Every card, sorted by name.
pub fn cards() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all()))
}

/// The card a name names, ignoring case.
pub fn card(name: &str) -> Option<CardId> {
    cards().find(name)
}

/// A game from decklists of names, top of the library first.
pub fn deal(decks: &[Vec<String>], seed: u64, shuffle: bool, opening_chooser: Option<PlayerId>) -> Result<Game, String> {
    deal_with(decks, &Outside::default(), seed, shuffle, opening_chooser)
}

/// What a deal has beyond its decks, by card name (`Setup`): each seat's sideboard - the cards it owns outside the game (CR
/// 100.4, 400.11a) - and companion chosen at deck building (CR 702.139a), the basic lands a Limited sideboard holds any number
/// of, and the format's minimum deck size (CR 100.2a-b).
#[derive(Clone, Copy, Debug)]
pub struct Outside<'a> {
    pub sideboards: &'a [Vec<String>],
    pub companions: &'a [Option<String>],
    pub basic_lands_outside: &'a [String],
    pub minimum_deck_size: u32,
}

/// Nothing outside the game, in a Freeform game: no minimum deck size, as no format names one.
impl Default for Outside<'_> {
    fn default() -> Self {
        Outside { sideboards: &[], companions: &[], basic_lands_outside: &[], minimum_deck_size: 0 }
    }
}

/// A game from decklists of names, top of the library first, and what each seat has outside the game (`Outside`).
pub fn deal_with(decks: &[Vec<String>], outside: &Outside, seed: u64, shuffle: bool, opening_chooser: Option<PlayerId>) -> Result<Game, String> {
    deal_on(cards(), decks, outside, seed, shuffle, opening_chooser)
}

/// `deal_with` on a card table of the caller's: every card and some test-only ones, for a Rust test that plays a scenario
/// with a card no printed card is (`cantrip_script::run_on`).
pub fn deal_on(
    table: &'static CardTable,
    decks: &[Vec<String>],
    outside: &Outside,
    seed: u64,
    shuffle: bool,
    opening_chooser: Option<PlayerId>,
) -> Result<Game, String> {
    let id = |name: &String| -> Result<CardId, String> {
        // Either face of a double-faced card names it (CR 711.1); a deck holds the front.
        let id = table.find(name).ok_or_else(|| format!("no card is named \"{name}\""))?;
        // A deck holds cards, never a token or an emblem, whose name is no card's (CR 201.4, 111.4: "Spirit Token").
        if !table.card_named[id.0 as usize] {
            return Err(format!("no card is named \"{name}\" (a token's or an emblem's name is no card's)"));
        }
        Ok(table.fronts[id.0 as usize].unwrap_or(id))
    };
    let ids = |names: &[String]| -> Result<Vec<CardId>, String> { names.iter().map(id).collect() };
    let setup = Setup {
        decks: decks.iter().map(|d| ids(d)).collect::<Result<_, _>>()?,
        sideboards: outside.sideboards.iter().map(|d| ids(d)).collect::<Result<_, _>>()?,
        basic_lands_outside: ids(outside.basic_lands_outside)?,
        companions: outside.companions.iter().map(|c| c.as_ref().map(id).transpose()).collect::<Result<_, _>>()?,
        minimum_deck_size: outside.minimum_deck_size,
        seed,
        shuffle,
        opening_chooser,
    };
    let mut game = Game::new(table, &setup);
    game.run();
    Ok(game)
}

/// A label's text, as a client shows it and a scenario names it.
pub fn label_text(label: &Label) -> &'static str {
    label_text_in(cards(), label)
}

/// `label_text` read from `table`: a game dealt on a table of its own (`run_on`, test-only cards among it) names its cards'
/// labels by that table's ids.
pub fn label_text_in(table: &'static CardTable, label: &Label) -> &'static str {
    let cards = || table;
    match *label {
        Label::Card(card, i) | Label::Option(card, i) => cards().get(card).labels.get(i as usize).copied().unwrap_or(""),
        Label::Name(card) | Label::Face(card) => cards().get(card).name,
        Label::Fused(_) => "Fuse: both halves",
        Label::TargetSlot(text, _) => text,
        Label::Pile { first: true, ref cards } if cards.is_empty() => "First pile: nothing",
        Label::Pile { first: true, .. } => "First pile",
        Label::Pile { first: false, ref cards } if cards.is_empty() => "Second pile: nothing",
        Label::Pile { first: false, .. } => "Second pile",
        Label::FaceUpPile { ref cards } if cards.is_empty() => "Face-up pile: nothing",
        Label::FaceUpPile { .. } => "Face-up pile",
        Label::FaceDownPile { count: 0 } => "Face-down pile: nothing",
        Label::FaceDownPile { .. } => "Face-down pile",
        // A modal spell's or ability's mode, from the card's own definition (its spell's, or the modal ability's, a reflexive
        // one among them).
        Label::Mode(ability, i) => cards().modal_of(ability).and_then(|m| m.modes.get(i as usize)).map_or("", |m| m.text),
        Label::Extra(card, i) => cards().get(card).additional_choice.get(i as usize).map_or("", |c| c.text),
        Label::PayLife(Color::White) => "Pay 2 life for {W/P}",
        Label::PayLife(Color::Blue) => "Pay 2 life for {U/P}",
        Label::PayLife(Color::Black) => "Pay 2 life for {B/P}",
        Label::PayLife(Color::Red) => "Pay 2 life for {R/P}",
        Label::PayLife(Color::Green) => "Pay 2 life for {G/P}",
        Label::PayGeneric(Color::White) => "Pay {2} for {2/W}",
        Label::PayGeneric(Color::Blue) => "Pay {2} for {2/U}",
        Label::PayGeneric(Color::Black) => "Pay {2} for {2/B}",
        Label::PayGeneric(Color::Red) => "Pay {2} for {2/R}",
        Label::PayGeneric(Color::Green) => "Pay {2} for {2/G}",
        Label::PayHybridAs(Mana::W, Mana::U, Mana::W) => "Pay {W/U} with {W}",
        Label::PayHybridAs(Mana::W, Mana::U, Mana::U) => "Pay {W/U} with {U}",
        Label::PayHybridAs(Mana::W, Mana::B, Mana::W) => "Pay {W/B} with {W}",
        Label::PayHybridAs(Mana::W, Mana::B, Mana::B) => "Pay {W/B} with {B}",
        Label::PayHybridAs(Mana::W, Mana::R, Mana::W) => "Pay {W/R} with {W}",
        Label::PayHybridAs(Mana::W, Mana::R, Mana::R) => "Pay {W/R} with {R}",
        Label::PayHybridAs(Mana::W, Mana::G, Mana::W) => "Pay {W/G} with {W}",
        Label::PayHybridAs(Mana::W, Mana::G, Mana::G) => "Pay {W/G} with {G}",
        Label::PayHybridAs(Mana::W, Mana::C, Mana::W) => "Pay {W/C} with {W}",
        Label::PayHybridAs(Mana::W, Mana::C, Mana::C) => "Pay {W/C} with {C}",
        Label::PayHybridAs(Mana::U, Mana::W, Mana::U) => "Pay {U/W} with {U}",
        Label::PayHybridAs(Mana::U, Mana::W, Mana::W) => "Pay {U/W} with {W}",
        Label::PayHybridAs(Mana::U, Mana::B, Mana::U) => "Pay {U/B} with {U}",
        Label::PayHybridAs(Mana::U, Mana::B, Mana::B) => "Pay {U/B} with {B}",
        Label::PayHybridAs(Mana::U, Mana::R, Mana::U) => "Pay {U/R} with {U}",
        Label::PayHybridAs(Mana::U, Mana::R, Mana::R) => "Pay {U/R} with {R}",
        Label::PayHybridAs(Mana::U, Mana::G, Mana::U) => "Pay {U/G} with {U}",
        Label::PayHybridAs(Mana::U, Mana::G, Mana::G) => "Pay {U/G} with {G}",
        Label::PayHybridAs(Mana::U, Mana::C, Mana::U) => "Pay {U/C} with {U}",
        Label::PayHybridAs(Mana::U, Mana::C, Mana::C) => "Pay {U/C} with {C}",
        Label::PayHybridAs(Mana::B, Mana::W, Mana::B) => "Pay {B/W} with {B}",
        Label::PayHybridAs(Mana::B, Mana::W, Mana::W) => "Pay {B/W} with {W}",
        Label::PayHybridAs(Mana::B, Mana::U, Mana::B) => "Pay {B/U} with {B}",
        Label::PayHybridAs(Mana::B, Mana::U, Mana::U) => "Pay {B/U} with {U}",
        Label::PayHybridAs(Mana::B, Mana::R, Mana::B) => "Pay {B/R} with {B}",
        Label::PayHybridAs(Mana::B, Mana::R, Mana::R) => "Pay {B/R} with {R}",
        Label::PayHybridAs(Mana::B, Mana::G, Mana::B) => "Pay {B/G} with {B}",
        Label::PayHybridAs(Mana::B, Mana::G, Mana::G) => "Pay {B/G} with {G}",
        Label::PayHybridAs(Mana::B, Mana::C, Mana::B) => "Pay {B/C} with {B}",
        Label::PayHybridAs(Mana::B, Mana::C, Mana::C) => "Pay {B/C} with {C}",
        Label::PayHybridAs(Mana::R, Mana::W, Mana::R) => "Pay {R/W} with {R}",
        Label::PayHybridAs(Mana::R, Mana::W, Mana::W) => "Pay {R/W} with {W}",
        Label::PayHybridAs(Mana::R, Mana::U, Mana::R) => "Pay {R/U} with {R}",
        Label::PayHybridAs(Mana::R, Mana::U, Mana::U) => "Pay {R/U} with {U}",
        Label::PayHybridAs(Mana::R, Mana::B, Mana::R) => "Pay {R/B} with {R}",
        Label::PayHybridAs(Mana::R, Mana::B, Mana::B) => "Pay {R/B} with {B}",
        Label::PayHybridAs(Mana::R, Mana::G, Mana::R) => "Pay {R/G} with {R}",
        Label::PayHybridAs(Mana::R, Mana::G, Mana::G) => "Pay {R/G} with {G}",
        Label::PayHybridAs(Mana::R, Mana::C, Mana::R) => "Pay {R/C} with {R}",
        Label::PayHybridAs(Mana::R, Mana::C, Mana::C) => "Pay {R/C} with {C}",
        Label::PayHybridAs(Mana::G, Mana::W, Mana::G) => "Pay {G/W} with {G}",
        Label::PayHybridAs(Mana::G, Mana::W, Mana::W) => "Pay {G/W} with {W}",
        Label::PayHybridAs(Mana::G, Mana::U, Mana::G) => "Pay {G/U} with {G}",
        Label::PayHybridAs(Mana::G, Mana::U, Mana::U) => "Pay {G/U} with {U}",
        Label::PayHybridAs(Mana::G, Mana::B, Mana::G) => "Pay {G/B} with {G}",
        Label::PayHybridAs(Mana::G, Mana::B, Mana::B) => "Pay {G/B} with {B}",
        Label::PayHybridAs(Mana::G, Mana::R, Mana::G) => "Pay {G/R} with {G}",
        Label::PayHybridAs(Mana::G, Mana::R, Mana::R) => "Pay {G/R} with {R}",
        Label::PayHybridAs(Mana::G, Mana::C, Mana::G) => "Pay {G/C} with {G}",
        Label::PayHybridAs(Mana::G, Mana::C, Mana::C) => "Pay {G/C} with {C}",
        Label::PayHybridAs(Mana::C, Mana::W, Mana::C) => "Pay {C/W} with {C}",
        Label::PayHybridAs(Mana::C, Mana::W, Mana::W) => "Pay {C/W} with {W}",
        Label::PayHybridAs(Mana::C, Mana::U, Mana::C) => "Pay {C/U} with {C}",
        Label::PayHybridAs(Mana::C, Mana::U, Mana::U) => "Pay {C/U} with {U}",
        Label::PayHybridAs(Mana::C, Mana::B, Mana::C) => "Pay {C/B} with {C}",
        Label::PayHybridAs(Mana::C, Mana::B, Mana::B) => "Pay {C/B} with {B}",
        Label::PayHybridAs(Mana::C, Mana::R, Mana::C) => "Pay {C/R} with {C}",
        Label::PayHybridAs(Mana::C, Mana::R, Mana::R) => "Pay {C/R} with {R}",
        Label::PayHybridAs(Mana::C, Mana::G, Mana::C) => "Pay {C/G} with {C}",
        Label::PayHybridAs(Mana::C, Mana::G, Mana::G) => "Pay {C/G} with {G}",
        Label::PayHybridAs(..) => "Pay the hybrid symbol with its other half",
        Label::ReduceAs(Mana::W, Mana::U, Mana::W) => "Reduce by {W} for {W/U}",
        Label::ReduceAs(Mana::W, Mana::U, Mana::U) => "Reduce by {U} for {W/U}",
        Label::ReduceAs(Mana::W, Mana::B, Mana::W) => "Reduce by {W} for {W/B}",
        Label::ReduceAs(Mana::W, Mana::B, Mana::B) => "Reduce by {B} for {W/B}",
        Label::ReduceAs(Mana::W, Mana::R, Mana::W) => "Reduce by {W} for {W/R}",
        Label::ReduceAs(Mana::W, Mana::R, Mana::R) => "Reduce by {R} for {W/R}",
        Label::ReduceAs(Mana::W, Mana::G, Mana::W) => "Reduce by {W} for {W/G}",
        Label::ReduceAs(Mana::W, Mana::G, Mana::G) => "Reduce by {G} for {W/G}",
        Label::ReduceAs(Mana::W, Mana::C, Mana::W) => "Reduce by {W} for {W/C}",
        Label::ReduceAs(Mana::W, Mana::C, Mana::C) => "Reduce by {C} for {W/C}",
        Label::ReduceAs(Mana::U, Mana::W, Mana::U) => "Reduce by {U} for {U/W}",
        Label::ReduceAs(Mana::U, Mana::W, Mana::W) => "Reduce by {W} for {U/W}",
        Label::ReduceAs(Mana::U, Mana::B, Mana::U) => "Reduce by {U} for {U/B}",
        Label::ReduceAs(Mana::U, Mana::B, Mana::B) => "Reduce by {B} for {U/B}",
        Label::ReduceAs(Mana::U, Mana::R, Mana::U) => "Reduce by {U} for {U/R}",
        Label::ReduceAs(Mana::U, Mana::R, Mana::R) => "Reduce by {R} for {U/R}",
        Label::ReduceAs(Mana::U, Mana::G, Mana::U) => "Reduce by {U} for {U/G}",
        Label::ReduceAs(Mana::U, Mana::G, Mana::G) => "Reduce by {G} for {U/G}",
        Label::ReduceAs(Mana::U, Mana::C, Mana::U) => "Reduce by {U} for {U/C}",
        Label::ReduceAs(Mana::U, Mana::C, Mana::C) => "Reduce by {C} for {U/C}",
        Label::ReduceAs(Mana::B, Mana::W, Mana::B) => "Reduce by {B} for {B/W}",
        Label::ReduceAs(Mana::B, Mana::W, Mana::W) => "Reduce by {W} for {B/W}",
        Label::ReduceAs(Mana::B, Mana::U, Mana::B) => "Reduce by {B} for {B/U}",
        Label::ReduceAs(Mana::B, Mana::U, Mana::U) => "Reduce by {U} for {B/U}",
        Label::ReduceAs(Mana::B, Mana::R, Mana::B) => "Reduce by {B} for {B/R}",
        Label::ReduceAs(Mana::B, Mana::R, Mana::R) => "Reduce by {R} for {B/R}",
        Label::ReduceAs(Mana::B, Mana::G, Mana::B) => "Reduce by {B} for {B/G}",
        Label::ReduceAs(Mana::B, Mana::G, Mana::G) => "Reduce by {G} for {B/G}",
        Label::ReduceAs(Mana::B, Mana::C, Mana::B) => "Reduce by {B} for {B/C}",
        Label::ReduceAs(Mana::B, Mana::C, Mana::C) => "Reduce by {C} for {B/C}",
        Label::ReduceAs(Mana::R, Mana::W, Mana::R) => "Reduce by {R} for {R/W}",
        Label::ReduceAs(Mana::R, Mana::W, Mana::W) => "Reduce by {W} for {R/W}",
        Label::ReduceAs(Mana::R, Mana::U, Mana::R) => "Reduce by {R} for {R/U}",
        Label::ReduceAs(Mana::R, Mana::U, Mana::U) => "Reduce by {U} for {R/U}",
        Label::ReduceAs(Mana::R, Mana::B, Mana::R) => "Reduce by {R} for {R/B}",
        Label::ReduceAs(Mana::R, Mana::B, Mana::B) => "Reduce by {B} for {R/B}",
        Label::ReduceAs(Mana::R, Mana::G, Mana::R) => "Reduce by {R} for {R/G}",
        Label::ReduceAs(Mana::R, Mana::G, Mana::G) => "Reduce by {G} for {R/G}",
        Label::ReduceAs(Mana::R, Mana::C, Mana::R) => "Reduce by {R} for {R/C}",
        Label::ReduceAs(Mana::R, Mana::C, Mana::C) => "Reduce by {C} for {R/C}",
        Label::ReduceAs(Mana::G, Mana::W, Mana::G) => "Reduce by {G} for {G/W}",
        Label::ReduceAs(Mana::G, Mana::W, Mana::W) => "Reduce by {W} for {G/W}",
        Label::ReduceAs(Mana::G, Mana::U, Mana::G) => "Reduce by {G} for {G/U}",
        Label::ReduceAs(Mana::G, Mana::U, Mana::U) => "Reduce by {U} for {G/U}",
        Label::ReduceAs(Mana::G, Mana::B, Mana::G) => "Reduce by {G} for {G/B}",
        Label::ReduceAs(Mana::G, Mana::B, Mana::B) => "Reduce by {B} for {G/B}",
        Label::ReduceAs(Mana::G, Mana::R, Mana::G) => "Reduce by {G} for {G/R}",
        Label::ReduceAs(Mana::G, Mana::R, Mana::R) => "Reduce by {R} for {G/R}",
        Label::ReduceAs(Mana::G, Mana::C, Mana::G) => "Reduce by {G} for {G/C}",
        Label::ReduceAs(Mana::G, Mana::C, Mana::C) => "Reduce by {C} for {G/C}",
        Label::ReduceAs(Mana::C, Mana::W, Mana::C) => "Reduce by {C} for {C/W}",
        Label::ReduceAs(Mana::C, Mana::W, Mana::W) => "Reduce by {W} for {C/W}",
        Label::ReduceAs(Mana::C, Mana::U, Mana::C) => "Reduce by {C} for {C/U}",
        Label::ReduceAs(Mana::C, Mana::U, Mana::U) => "Reduce by {U} for {C/U}",
        Label::ReduceAs(Mana::C, Mana::B, Mana::C) => "Reduce by {C} for {C/B}",
        Label::ReduceAs(Mana::C, Mana::B, Mana::B) => "Reduce by {B} for {C/B}",
        Label::ReduceAs(Mana::C, Mana::R, Mana::C) => "Reduce by {C} for {C/R}",
        Label::ReduceAs(Mana::C, Mana::R, Mana::R) => "Reduce by {R} for {C/R}",
        Label::ReduceAs(Mana::C, Mana::G, Mana::C) => "Reduce by {C} for {C/G}",
        Label::ReduceAs(Mana::C, Mana::G, Mana::G) => "Reduce by {G} for {C/G}",
        Label::ReduceAs(..) => "Reduce by the hybrid symbol's other half",
        Label::ReduceGeneric(Color::White) => "Reduce by {2} for {2/W}",
        Label::ReduceGeneric(Color::Blue) => "Reduce by {2} for {2/U}",
        Label::ReduceGeneric(Color::Black) => "Reduce by {2} for {2/B}",
        Label::ReduceGeneric(Color::Red) => "Reduce by {2} for {2/R}",
        Label::ReduceGeneric(Color::Green) => "Reduce by {2} for {2/G}",
        Label::Counter(kind) => kind.name(),
        Label::ManaType(mana) => match mana {
            Mana::W => "{W}",
            Mana::U => "{U}",
            Mana::B => "{B}",
            Mana::R => "{R}",
            Mana::G => "{G}",
            Mana::C => "{C}",
        },
        Label::Miracle(m) if m.is_own() => "Its own miracle",
        Label::Miracle(m) => cards().get(m.granter).granted_miracles.get(m.index as usize).map_or("", |g| g.text),
        Label::ManaCost => "Pay its mana cost",
        Label::AlternativeCost(card, i) => cards().get(card).spell_alternatives.get(i as usize).map_or("", |a| a.text),
        Label::Way(card, i) => cards().get(card).ways.get(i as usize).map_or("", |w| w.text),
        Label::CastPermission { own: true, .. } => "Its own permission",
        Label::CastPermission { by: Some(card), .. } => cards().get(card).name,
        Label::CastPermission { by: None, .. } => "Its own speed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "If you cast it any time a sorcery couldn't have been cast, ..." (`CardDef::untimely_cleanup_rider`) is about a cast by
    /// the card's own "you may cast this spell as though it had flash", which every card that prints it prints first
    /// (Weatherlight's cycle): a card with the rider and not the permission would never be cast by its own. And a card has
    /// the rider exactly when its text has the sentence, so none can forget it or carry it unprinted.
    #[test]
    fn a_cleanup_rider_comes_with_its_own_flash_permission() {
        const RIDER: &str = "If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.";
        for def in &cards().cards {
            assert!(!def.untimely_cleanup_rider || def.cast_as_though_flash, "{}: a cleanup rider without its own flash permission", def.name);
            assert_eq!(def.untimely_cleanup_rider, def.text.contains(RIDER), "{}: the cleanup rider and its text disagree", def.name);
        }
    }

    /// Craft's materials (CR 702.167a) are exiled by activating an ability, which exiles its source with them: no way of
    /// casting a spell pays them, so a card whose spell costs carry them, or whose craft ability does not exile itself, is
    /// refused here.
    #[test]
    fn only_an_activated_ability_that_exiles_its_source_exiles_materials() {
        for def in &cards().cards {
            for ability in def.abilities.iter().chain(def.delayed.iter()) {
                if let AbilityDef::Activated(a) = ability {
                    assert!(a.cost.materials.is_none() || a.cost.exile_self, "{}: craft exiles its source with its materials", def.name);
                }
            }
            let spell_costs = def.ways.iter().map(|w| w.cost).chain(def.additional_cost).chain(def.optional_cost.map(|o| o.cost));
            for cost in spell_costs.chain(def.additional_choice.iter().map(|c| c.cost)) {
                assert!(cost.materials.is_none(), "{}: a spell's cost can't exile craft materials", def.name);
            }
        }
    }

    /// A creature whose power or toughness is printed as a star has it from a characteristic-defining ability (CR 208.2a), and
    /// once that is lost (CR 604.3, 613.1f) the star is worth 0 (CR 208.5): `StaticDef::lost_pt` says what it leaves, so the next
    /// card with a star cannot forget it.
    #[test]
    fn every_star_creature_says_what_its_power_and_toughness_are_once_the_ability_is_lost() {
        for def in &cards().cards {
            if !def.is(CardType::Creature) || (def.power.is_some() && def.toughness.is_some()) {
                continue;
            }
            for st in def.statics.iter().filter(|s| s.own && s.layer == Layer::PtDefine) {
                assert!(st.lost_pt.is_some(), "{}: its power and toughness are defined by an ability, and `.lost_pt(power, toughness)` is missing", def.name);
            }
        }
    }
}
