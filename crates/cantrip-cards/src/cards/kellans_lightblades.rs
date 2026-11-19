use cantrip_api::*;
use cantrip_mechanics::{bargain, effects, targets};
use cantrip_oracle as oracle;

/// "Kellan's Lightblades deals 3 damage to target attacking or blocking creature. If this spell was bargained, destroy that
/// creature instead." Which of the two it does is read from the spell as it resolves (CR 702.166c); with its one target illegal
/// the spell does not resolve at all (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if bargain::bargained(item) {
        effects::destroy_first_target(g, item)
    } else {
        effects::damage_first_target::<3>(g, item)
    }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ATTACKING_OR_BLOCKING_CREATURE], effect),
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    ..oracle::KELLANS_LIGHTBLADES
};
