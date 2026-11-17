use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS_SUBJECT,
        effects::target_creature_gets::<-3, 0>,
        "Whenever this creature attacks, target creature defending player controls gets -3/-0 until end of turn.",
    )
    .targets(&[targets::CREATURE_DEFENDING_PLAYER_CONTROLS])],
    ..oracle::SPRING_SPLASHER
};
