//! Mobilize N (CR 702.181a): "Whenever this creature attacks, create N tapped and attacking 1/1 red Warrior creature
//! tokens. Sacrifice them at the beginning of the next end step." Voice of Victory prints it with 2.
//!
//! A card with mobilize lists `trigger::<N>(text)` among its `abilities`, `SACRIFICE` as its first `delayed` ability
//! and `WARRIOR` among its `EXTRA` definitions.

use cantrip_api::*;

/// The 1/1 red Warrior creature token mobilize makes.
pub static WARRIOR: CardDef = CardDef {
    name: "Warrior Token",
    token_name: Some(&[Subtype::Warrior]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Warrior]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// `N` tapped and attacking Warriors, then sacrificed together at the beginning of the next end step (CR 603.7c, 701.21a).
/// Mobilize does not say what they attack, so their controller chooses for each one, a player, a planeswalker or a battle,
/// not necessarily what the creature with mobilize attacks (CR 508.4), asked only where there is a choice and before any is
/// made; the ones attacking the same thing are created together (CR 101.4). With nothing to attack they are still created,
/// tapped. The sacrifice is the card's delayed ability 0; it remembers every token that was created, however many a
/// replacement made of them (Elspeth, Storm Slayer doubles them).
fn create_warriors<const N: u8>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let warrior = g.card_id(&WARRIOR);
    let you = item.controller;
    let mut chosen: Vec<Option<Target>> = Vec::with_capacity(N as usize);
    for _ in 0..N {
        chosen.push(crate::combat::choose_attack_target(g, you, item.object)?);
    }
    let mut created = Vec::new();
    let mut done: Vec<Option<Target>> = Vec::new();
    for &defender in &chosen {
        if done.contains(&defender) {
            continue;
        }
        done.push(defender);
        let n = chosen.iter().filter(|&&d| d == defender).count();
        // The Warriors are the keyword's (CR 702.181a), whatever the card's text says now.
        created.extend(g.create_tokens_worded(&vec![(warrior, you); n], true, defender, Words::NONE)?);
    }
    crate::delayed::at_next_end_step_about(g, item, 0, &created);
    Ok(())
}

/// Mobilize N itself; `text` is the card's own reminder text.
pub const fn trigger<const N: u8>(text: &'static str) -> AbilityDef {
    triggered(crate::triggers::THIS_ATTACKS, create_warriors::<N>, text)
}

pub const SACRIFICE: AbilityDef = delayed_trigger(crate::delayed::sacrifice_created, "Sacrifice the tokens created by mobilize.");
