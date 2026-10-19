//! The engine for its users (the scenario runner, the benchmarks, later the host and the Python bindings): the rules,
//! and the card table every card crate contributes to, built once per process and never copied.

use std::sync::OnceLock;

pub mod undo;
pub mod visibility;

pub use cantrip_api::*;
pub use cantrip_rules::{Game, Rejected, Setup, activation_costs, cast_costs, cast_total, exile_cast_texts, granted_cast_cost, life_for_cast, chars, check_answer, describe, distribution_problem, live, name_problem, summoning_sick, granted_cast_total, special_action_costs, sacrifices_for_reduction};

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
    deal_with(decks, &[], &[], seed, shuffle, opening_chooser)
}

/// A game from decklists of names, top of the library first, with each seat's sideboard - the cards it owns outside the
/// game (CR 100.4, 400.11a) - and, for a Limited game, the basic lands a sideboard holds any number of (`Setup`).
pub fn deal_with(
    decks: &[Vec<String>],
    sideboards: &[Vec<String>],
    basic_lands_outside: &[String],
    seed: u64,
    shuffle: bool,
    opening_chooser: Option<PlayerId>,
) -> Result<Game, String> {
    let ids = |names: &[String]| -> Result<Vec<CardId>, String> {
        names
            .iter()
            .map(|name| {
                // Either face of a double-faced card names it (CR 711.1); a deck holds the front.
                let id = card(name).ok_or_else(|| format!("no card is named \"{name}\""))?;
                Ok(cards().fronts[id.0 as usize].unwrap_or(id))
            })
            .collect()
    };
    let setup = Setup {
        decks: decks.iter().map(|d| ids(d)).collect::<Result<_, _>>()?,
        sideboards: sideboards.iter().map(|d| ids(d)).collect::<Result<_, _>>()?,
        basic_lands_outside: ids(basic_lands_outside)?,
        seed,
        shuffle,
        opening_chooser,
    };
    let mut game = Game::new(cards(), &setup);
    game.run();
    Ok(game)
}

/// A label's text, as a client shows it and a scenario names it.
pub fn label_text(label: &Label) -> &'static str {
    match *label {
        Label::Card(card, i) => cards().get(card).labels.get(i as usize).copied().unwrap_or(""),
        Label::Name(card) | Label::Face(card) => cards().get(card).name,
        Label::Pile { first: true, empty: true } => "First pile: nothing",
        Label::Pile { first: true, empty: false } => "First pile",
        Label::Pile { first: false, empty: true } => "Second pile: nothing",
        Label::Pile { first: false, empty: false } => "Second pile",
        // A modal spell's or ability's mode, from the card's own definition (its spell's, or its first modal ability's).
        Label::Mode(ability, i) => {
            let def = cards().get(ability.card);
            let modal = if ability.index == u8::MAX {
                def.modal
            } else {
                match def.abilities.get(ability.index as usize) {
                    Some(AbilityDef::Activated(a)) => a.modal,
                    Some(AbilityDef::Triggered(t)) => t.modal,
                    _ => None,
                }
            };
            modal.and_then(|m| m.modes.get(i as usize)).map_or("", |m| m.text)
        }
        Label::Extra(card, i) => cards().get(card).additional_choice.get(i as usize).map_or("", |c| c.text),
        Label::PayLife(Color::White) => "Pay 2 life for {W/P}",
        Label::PayLife(Color::Blue) => "Pay 2 life for {U/P}",
        Label::PayLife(Color::Black) => "Pay 2 life for {B/P}",
        Label::PayLife(Color::Red) => "Pay 2 life for {R/P}",
        Label::PayLife(Color::Green) => "Pay 2 life for {G/P}",
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Activating an ability (CR 602.2) pays neither behold nor a cost of tapped permanents chosen once the mana is paid
    /// (web-slinging's): those are built for spells only, so a card whose activated ability has one is refused here, before any
    /// game reaches the assertion in `activate`.
    #[test]
    fn every_activated_ability_has_a_cost_activation_can_pay() {
        for def in &cards().cards {
            for ability in def.abilities.iter().chain(def.delayed.iter()) {
                if let AbilityDef::Activated(a) = ability {
                    assert!(
                        a.cost.behold.is_none() && !a.cost.permanents.is_some_and(|p| p.tapped),
                        "{}: an activated ability's behold or cost of tapped permanents is not built",
                        def.name
                    );
                }
            }
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
