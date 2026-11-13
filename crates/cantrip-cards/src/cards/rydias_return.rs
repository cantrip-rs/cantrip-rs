use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::creatures_you_control_get::<3, 3>, "Creatures you control get +3/+3 until end of turn."),
    mode(effects::return_each_target_to_hand, "Return up to two target permanent cards from your graveyard to your hand.")
        .targets(&[up_to(2, targets::PERMANENT_CARD_IN_YOUR_GRAVEYARD.filter)]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::RYDIAS_RETURN };
