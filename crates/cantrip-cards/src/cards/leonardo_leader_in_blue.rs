use cantrip_api::*;
use cantrip_mechanics::effects::{creatures_you_control_get, this_gains_first_strike};
use cantrip_mechanics::sneak;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "If his sneak cost was paid" (CR 702.190a, 603.4): an intervening "if", asked as the ability triggers and again as it
/// resolves, of the permanent the sneak spell became. "Creatures you control get +2/+0 until end of turn" is
/// `creatures_you_control_get`: the creatures as it resolves, and no creature that enters later (CR 611.2c).
pub static CARD: CardDef = CardDef {
    ways: &[sneak::way(mana("{3}{W}{W}"), "Sneak {3}{W}{W}")],
    abilities: &[
        triggered(THIS_ENTERS, creatures_you_control_get::<2, 0>, "When Leonardo enters, if his sneak cost was paid, creatures you control get +2/+0 until end of turn.")
            .condition(|q, asker| sneak::was_cast_for_sneak(q, asker.source)),
        ability(cost("{1}{W}"), this_gains_first_strike, "{1}{W}: Leonardo gains first strike until end of turn."),
    ],
    ..oracle::LEONARDO_LEADER_IN_BLUE
};
