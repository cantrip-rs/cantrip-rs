use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Untap target creature. It gets +1/+0 and gains indestructible until end of turn." The one target is untapped first, then
/// pumped and made indestructible (CR 608.2c); nothing happens to it if it is no longer a legal target (CR 608.2b).
fn untap_and_bolster(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::untap_targets(g, item)?;
    effects::target_creature_gets_and_gains_indestructible::<1, 0>(g, item)
}

static MODAL: ModalDef = modes(&[
    mode(untap_and_bolster, "Untap target creature. It gets +1/+0 and gains indestructible until end of turn.").targets(&[targets::CREATURE]),
    mode(effects::destroy_first_target, "Destroy target creature with power 4 or greater.").targets(&[targets::CREATURE_WITH_POWER_4_OR_GREATER]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::MOMENT_OF_VALOR };
