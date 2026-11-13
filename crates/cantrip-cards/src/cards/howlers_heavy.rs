use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::cycling(mana("{1}{U}"), "Cycling {1}{U} ({1}{U}, Discard this card: Draw a card.)"),
        triggered(
            triggers::YOU_CYCLE_THIS_CARD,
            effects::target_creature_gets::<-3, 0>,
            "When you cycle this card, target creature or Vehicle an opponent controls gets -3/-0 until end of turn.",
        )
        .targets(&[targets::CREATURE_OR_VEHICLE_AN_OPPONENT_CONTROLS]),
    ],
    ..oracle::HOWLERS_HEAVY
};
