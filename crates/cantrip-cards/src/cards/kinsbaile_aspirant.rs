use cantrip_api::*;
use cantrip_mechanics::{behold, effects, triggers::ANOTHER_CREATURE_YOU_CONTROL_ENTERS};
use cantrip_oracle as oracle;

/// "As an additional cost to cast this spell, behold a Kithkin or pay {2}" (CR 601.2b, 701.4a): one of the two, chosen as
/// the spell is cast. "Whenever another creature you control enters, this creature gets +1/+1 until end of turn" (CR 603.6a).
pub static CARD: CardDef = CardDef {
    additional_choice: &[
        ExtraCost { text: "Behold a Kithkin", cost: behold::cost(behold::kithkin, "Kithkin") },
        ExtraCost { text: "Pay {2}", cost: Cost { mana: mana("{2}"), ..Cost::NONE } },
    ],
    abilities: &[triggered(
        ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
        effects::this_gets::<1, 1>,
        "Whenever another creature you control enters, this creature gets +1/+1 until end of turn.",
    )],
    ..oracle::KINSBAILE_ASPIRANT
};
