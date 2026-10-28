use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

/// "Target creature with power 2 or less" (CR 115.1).
fn power_two_or_less(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id)
        && q.chars(id).is(CardType::Creature) && q.chars(id).power.is_some_and(|p| p <= 2))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        lands::SACRIFICE_TO_FETCH_BASIC_TAPPED,
        ability(TAP.and(SACRIFICE_THIS), effects::target_creature_cant_be_blocked_this_turn, "{T}, Sacrifice this land: Target creature with power 2 or less can't be blocked this turn.")
            .targets(&[target(power_two_or_less)]),
    ],
    ..oracle::ESCAPE_TUNNEL
};
