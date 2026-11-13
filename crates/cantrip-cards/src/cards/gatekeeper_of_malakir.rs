use cantrip_api::*;
use cantrip_mechanics::{kicker, target_player_sacrifices_a_creature, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    kicker: kicker(mana("{B}")),
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        target_player_sacrifices_a_creature,
        "When this creature enters, if it was kicked, target player sacrifices a creature of their choice.",
    )
    .targets(&[targets::PLAYER])
    .condition(kicker::was_kicked)],
    ..oracle::GATEKEEPER_OF_MALAKIR
};
