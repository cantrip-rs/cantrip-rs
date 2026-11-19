use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Tap up to one target creature. Scry 1, then draw a card." With no target, or one no longer legal, nothing is tapped but the
/// rest still happens (CR 608.2b).
fn tap_scry_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::tap_targets(g, item)?;
    effects::scry_then_draw::<1>(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE.optional()], tap_scry_and_draw), ..oracle::PLUNGE_INTO_WINTER };
