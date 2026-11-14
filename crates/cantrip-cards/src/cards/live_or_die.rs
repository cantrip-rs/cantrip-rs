use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Choose one" (CR 700.2a); return the card under your control (CR 110.2a) or destroy (CR 701.8).
static MODAL: ModalDef = modes(&[
    mode(effects::return_first_target_to_battlefield, "Return target creature card from your graveyard to the battlefield.")
        .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
    mode(effects::destroy_first_target, "Destroy target creature.").targets(&[targets::CREATURE]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::LIVE_OR_DIE };
