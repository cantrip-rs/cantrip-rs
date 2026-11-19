use cantrip_api::*;
use cantrip_mechanics::{lands, targets};
use cantrip_oracle as oracle;

/// "{T}: Add {C}.", "{1}, {T}: Add one mana of any color." and "{4}, {T}, Sacrifice this land: Put two +1/+1 counters on target
/// creature. Activate only as a sorcery." (CR 605.1a, 602.5d, 122.1), the last as the Hobbit cycle prints it
/// (`lands::sacrifice_for_two_counters`) with any creature as its target.
pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        lands::ANY_COLOR_FOR_ONE,
        lands::sacrifice_for_two_counters(
            "{4}, {T}",
            &[targets::CREATURE],
            "{4}, {T}, Sacrifice this land: Put two +1/+1 counters on target creature. Activate only as a sorcery.",
        ),
    ],
    ..oracle::CAPTIVATING_CAVE
};
