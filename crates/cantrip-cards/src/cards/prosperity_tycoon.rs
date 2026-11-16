use cantrip_api::*;
use cantrip_mechanics::effects::this_gains_indestructible_then_tap;
use cantrip_mechanics::{targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "{2}, Sacrifice a token: This creature gains indestructible until end of turn. Tap it." The token is sacrificed as a cost
/// (CR 118.3, 701.21a), so it is gone before the ability can be responded to; the creature is tapped after it gains indestructible.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            tokens::create_a_mercenary,
            "When this creature enters, create a 1/1 red Mercenary creature token with \"{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.\"",
        ),
        ability(
            cost("{2}").and(sacrifice(1, targets::token_you_control)),
            this_gains_indestructible_then_tap,
            "{2}, Sacrifice a token: This creature gains indestructible until end of turn. Tap it.",
        ),
    ],
    ..oracle::PROSPERITY_TYCOON
};

pub static EXTRA: &[&CardDef] = &[&tokens::MERCENARY];
