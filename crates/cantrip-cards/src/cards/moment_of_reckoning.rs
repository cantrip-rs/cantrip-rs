use cantrip_api::*;
use cantrip_mechanics::{destroy_target, effects, targets};
use cantrip_oracle as oracle;

/// Mode 1: "Destroy target nonland permanent" (CR 701.8, 608.2b): each instance has a target of its own.
fn destroy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

/// "Choose up to four. You may choose the same mode more than once" (CR 700.2, 700.2d): none to four choices, each an instance of
/// its own with its own target, run in printed order as the spell resolves.
static MODAL: ModalDef = modes(&[
    mode(destroy, "Destroy target nonland permanent.").targets(&[targets::NONLAND_PERMANENT]),
    mode(effects::return_target_card_to_battlefield, "Return target nonland permanent card from your graveyard to the battlefield.")
        .targets(&[targets::NONLAND_PERMANENT_CARD_IN_YOUR_GRAVEYARD]),
])
.choose(0, 4)
.repeat();

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::MOMENT_OF_RECKONING };
