use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Choose one" (CR 700.2a): destroy (CR 701.8), or return the Zombie card under your control tapped.
static MODAL: ModalDef = modes(&[
    mode(effects::destroy_first_target, "Destroy target creature or planeswalker.").targets(&[targets::CREATURE_OR_PLANESWALKER]),
    mode(effects::return_first_target_to_battlefield_tapped, "Return target Zombie creature card from your graveyard to the battlefield tapped.").targets(&[targets::ZOMBIE_CREATURE_CARD_IN_YOUR_GRAVEYARD]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::DEADLY_PLOT };
