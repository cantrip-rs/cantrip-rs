use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::PROWESS};
use cantrip_oracle as oracle;

/// "{R}, Sacrifice this creature: It deals damage equal to its power to target creature or planeswalker." The sacrifice is a
/// cost (CR 601.2h), so the creature is gone when this resolves and deals damage equal to its power as it last existed on the
/// battlefield, prowess pumps included (CR 113.7a, 608.2h).
pub static CARD: CardDef = CardDef {
    abilities: &[
        PROWESS,
        ability(
            cost("{R}").and(SACRIFICE_THIS),
            effects::this_deals_damage_equal_to_its_power_to_first_target,
            "{R}, Sacrifice this creature: It deals damage equal to its power to target creature or planeswalker.",
        )
        .targets(&[targets::CREATURE_OR_PLANESWALKER]),
    ],
    ..oracle::HEARTFIRE_IMMOLATOR
};
