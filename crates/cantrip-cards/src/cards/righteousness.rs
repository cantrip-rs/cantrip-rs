use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target blocking creature gets +7/+7 until end of turn." (CR 115.1).
fn target_blocking_creature_gets_7_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if targets::permanent(q, id) && q.is_blocking(id) && q.chars(id).is(CardType::Creature)
    )
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[target(target_blocking_creature_gets_7_target_0)],
        effects::target_creature_gets::<7, 7>,
    ),
    ..oracle::RIGHTEOUSNESS
};
