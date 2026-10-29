use cantrip_api::*;
use cantrip_mechanics::{behold, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature an opponent controls with power 2 or less" (CR 115.1, 208.1): its power as the layers make it, as it is chosen
/// and again as the ability resolves.
const CREATURE_AN_OPPONENT_CONTROLS_WITH_POWER_2_OR_LESS: TargetSpec = target(targets::creature_an_opponent_controls_with_power_at_most::<2>);

/// "As an additional cost to cast this spell, behold a Goblin or pay {2}" (CR 601.2b, 701.4a). "This creature can't block" is
/// a keyword; "When this creature dies, destroy target creature an opponent controls with power 2 or less" (CR 700.4, 701.8).
pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::CantBlock]),
    additional_choice: &[
        ExtraCost { text: "Behold a Goblin", cost: behold::cost(behold::goblin, "Goblin") },
        ExtraCost { text: "Pay {2}", cost: Cost { mana: mana("{2}"), ..Cost::NONE } },
    ],
    abilities: &[triggered(
        triggers::THIS_DIES,
        effects::destroy_first_target,
        "When this creature dies, destroy target creature an opponent controls with power 2 or less.",
    )
    .targets(&[CREATURE_AN_OPPONENT_CONTROLS_WITH_POWER_2_OR_LESS])],
    ..oracle::MUDBUTTON_CURSETOSSER
};
