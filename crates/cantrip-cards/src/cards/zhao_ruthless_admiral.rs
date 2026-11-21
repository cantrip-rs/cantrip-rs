use cantrip_api::*;
use cantrip_mechanics::{effects, firebending, triggers};
use cantrip_oracle as oracle;

/// "Whenever you sacrifice another permanent, creatures you control get +1/+0 until end of turn" (CR 701.21a, 611.2c): one trigger for
/// each permanent sacrificed, each pumping the creatures its controller controls as it resolves, Zhao among them.
pub static CARD: CardDef = CardDef {
    abilities: &[
        firebending::trigger::<2>("Firebending 2 (Whenever this creature attacks, add {R}{R}. This mana lasts until end of combat.)"),
        triggered(
            triggers::YOU_SACRIFICE_ANOTHER_PERMANENT,
            effects::creatures_you_control_get::<1, 0>,
            "Whenever you sacrifice another permanent, creatures you control get +1/+0 until end of turn.",
        )
        .on_sacrifice(),
    ],
    ..oracle::ZHAO_RUTHLESS_ADMIRAL
};
