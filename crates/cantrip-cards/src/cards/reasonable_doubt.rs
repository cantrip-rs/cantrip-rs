use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Counter target spell unless its controller pays {2}. Suspect up to one target creature." (CR 701.6, 118.12, 701.60a): the two
/// targets are chosen together as it is cast, and each part checks its own again as it resolves (CR 608.2b). With the spell no longer
/// a legal target the counter does nothing, but the creature, if still legal, is suspected; with the creature gone the spell is still
/// countered unless its controller pays. The creature is optional ("up to one"), but not the spell: it can't be cast just to suspect.
fn counter_unless_pays_and_suspect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::counter_first_target_unless_pays::<2>(g, item)?;
    effects::suspect_target::<1>(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::SPELL, targets::CREATURE.optional()], counter_unless_pays_and_suspect), ..oracle::REASONABLE_DOUBT };
