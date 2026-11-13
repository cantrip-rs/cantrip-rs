use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::creatures_target_player_controls_get::<0, -1, -1>, "Creatures target opponent controls get -1/-1 until end of turn.").targets(&[targets::OPPONENT]),
    mode(effects::return_each_target_to_hand, "Return up to two target creature cards from your graveyard to your hand.")
        .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.between(0, 2)]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::WAIL_OF_WAR };
