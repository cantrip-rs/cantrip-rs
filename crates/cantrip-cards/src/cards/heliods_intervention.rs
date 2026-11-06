use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target player gains twice X life." (CR 608.2, 119.3, 107.3).
fn target_player_gains_twice_x_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_player_gains_life(g, item, 2 * item.x as i32)
}

static MODES: ModalDef = modes(&[
    mode(effects::destroy_each_target, "Destroy X target artifacts and/or enchantments.").targets(&[targets::ARTIFACT_OR_ENCHANTMENT.counted()]),
    mode(target_player_gains_twice_x_life, "Target player gains twice X life.").targets(&[targets::PLAYER]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODES), ..oracle::HELIODS_INTERVENTION };
