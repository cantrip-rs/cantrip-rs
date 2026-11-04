use cantrip_api::*;
use cantrip_mechanics::{pump, targets};
use cantrip_oracle as oracle;

/// "{1}{G}: Target blocking creature gets +0/+1 until end of turn." (CR 115.1).
fn target_blocking_creature_gets_0_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if targets::permanent(q, id) && q.is_blocking(id) && q.chars(id).is(CardType::Creature)
    )
}

/// "{1}{G}: Target blocking creature gets +0/+1 until end of turn." (CR 608.2, 611.2a).
fn target_blocking_creature_gets_0(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    pump(g, item.object, target, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}{G}"),
                target_blocking_creature_gets_0,
                "{1}{G}: Target blocking creature gets +0/+1 until end of turn.",
            )
            .targets(&[target(target_blocking_creature_gets_0_target_0)]),
    ],
    ..oracle::ELVEN_FORTRESS
};
