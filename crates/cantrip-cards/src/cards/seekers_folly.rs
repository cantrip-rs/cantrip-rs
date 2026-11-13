use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::target_player_discards::<2>, "Target opponent discards two cards.").targets(&[targets::OPPONENT]),
    mode(effects::creatures_your_opponents_control_get::<-1, -1>, "Creatures your opponents control get -1/-1 until end of turn."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::SEEKERS_FOLLY };
