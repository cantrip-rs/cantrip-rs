use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, targets};
use cantrip_oracle as oracle;

/// "Unsubtle Mockery deals 4 damage to target creature. Surveil 1." With the target illegal as it resolves the spell does not
/// resolve at all (CR 608.2b), so there is no surveil; a legal target that is dealt no damage (prevented) still lets it surveil.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 4);
    g.end_sentence();
    effects::surveil(g, item, 1)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::UNSUBTLE_MOCKERY };
