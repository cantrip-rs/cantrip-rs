use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::target_player_draws_and_loses_life::<2, 2>, "Target player draws two cards and loses 2 life.").targets(&[targets::PLAYER]),
    mode(effects::target_creature_gets_and_gains_lifelink::<2, 2>, "Target creature gets +2/+2 and gains lifelink until end of turn.").targets(&[targets::CREATURE]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::REVERENT_HOWL };
