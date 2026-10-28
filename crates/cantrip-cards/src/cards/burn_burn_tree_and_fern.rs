use cantrip_api::*;
use cantrip_mechanics::{damage_target, destroy_target, targets};
use cantrip_oracle as oracle;

/// I - "This Saga deals 6 damage to target creature an opponent controls." The Saga is the source (CR 120.3); with its
/// target gone the chapter does not resolve (CR 608.2b).
fn six_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 6);
    Ok(())
}

/// II - "Destroy target artifact an opponent controls."
fn destroy_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

/// III, IV - "Add {R}" (CR 106.4): the chapter triggers as the Saga's lore counter is added in the precombat main phase
/// (CR 714.3b), so the mana can be spent there; it empties at the end of that phase.
fn add_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::R, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, six_damage, "I \u{2014} This Saga deals 6 damage to target creature an opponent controls.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        chapter(2, destroy_artifact, "II \u{2014} Destroy target artifact an opponent controls.")
            .targets(&[targets::ARTIFACT_AN_OPPONENT_CONTROLS]),
        chapter(3, add_red, "III, IV \u{2014} Add {R}."),
        chapter(4, add_red, "III, IV \u{2014} Add {R}."),
    ],
    ..oracle::BURN_BURN_TREE_AND_FERN
};
