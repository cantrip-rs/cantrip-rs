use cantrip_api::*;
use cantrip_mechanics::{exile_target, targets::card_in_graveyard};
use cantrip_oracle as oracle;

/// An artifact card in any graveyard (CR 115.1).
fn artifact_card(q: &dyn Query, _: &Asker, t: Target) -> bool {
    card_in_graveyard(q, t, &[CardType::Artifact])
}

/// Exile the target, then gain 2 life (CR 608.2b, 701.13, 119.3).
fn rob(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if exile_target(g, item, 0) {
        g.end_sentence();
        g.gain_life(item.controller, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{B}").and(TAP), rob, "{B}, {T}: Exile target artifact card from a graveyard. You gain 2 life.")
        .targets(&[target(artifact_card)])],
    ..oracle::GRAVE_ROBBERS
};
