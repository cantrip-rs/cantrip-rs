use cantrip_api::*;
use cantrip_mechanics::{effects, power_up};
use cantrip_oracle as oracle;

/// "Each power-up ability of permanents you control can be activated an additional time" (CR 702.193a, 602.5b) is the core's
/// `extra_power_up_activations`. "Put two +1/+1 counters on Wonder Man" is his own power-up. Flying is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[power_up::ability(
        cost("{5}{R}{R}"),
        effects::two_plus_one_counters_on_this,
        "Power-up \u{2014} {5}{R}{R}: Put two +1/+1 counters on Wonder Man. (Activate each power-up ability only . . . once? Reduce the cost by his mana cost if he entered this turn.)",
    )],
    extra_power_up_activations: 1,
    ..oracle::WONDER_MAN_HOLLYWOOD_HERO
};
