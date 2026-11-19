use cantrip_api::*;
use cantrip_mechanics::{bargain, effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets -3/-3 until end of turn. If this spell was bargained, that creature gets -5/-5 until end of turn
/// instead." (CR 611.2a, 608.2b, 702.166b): a bargained spell gives -5/-5 and no -3/-3; the target must still be legal as the spell
/// resolves.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if bargain::bargained(item) {
        effects::target_creature_gets::<-5, -5>(g, item)
    } else {
        effects::target_creature_gets::<-3, -3>(g, item)
    }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    ..oracle::CANDY_GRAPPLE
};
