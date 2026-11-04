use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on target creature you control. It gains indestructible until end of turn." Both happen only to a
/// target still legal as the spell resolves (CR 608.2b).
fn counter_and_indestructible(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_target_and_gain_until_end_of_turn(g, item, Keyword::Indestructible)
}

static MODAL: ModalDef = modes(&[
    mode(effects::destroy_first_target, "Destroy target artifact or enchantment.").targets(&[targets::ARTIFACT_OR_ENCHANTMENT]),
    mode(counter_and_indestructible, "Put a +1/+1 counter on target creature you control. It gains indestructible until end of turn.")
        .targets(&[targets::CREATURE_YOU_CONTROL]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::ORIGIN_OF_METALBENDING };
