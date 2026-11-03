use cantrip_api::*;
use cantrip_mechanics::{pump, targets};
use cantrip_oracle as oracle;

/// "{1}{R}: Target Dwarf creature gets +1/+0 until end of turn." (CR 115.1).
fn target_dwarf_creature_gets_1_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Dwarf)) && q
        .chars(id).is(CardType::Creature)))
    )
}

/// "{1}{R}: Target Dwarf creature gets +1/+0 until end of turn." (CR 608.2, 611.2a).
fn target_dwarf_creature_gets_1(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    pump(g, item.object, target, 1, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}{R}"),
                target_dwarf_creature_gets_1,
                "{1}{R}: Target Dwarf creature gets +1/+0 until end of turn.",
            )
            .targets(&[target(target_dwarf_creature_gets_1_target_0)]),
    ],
    ..oracle::DWARVEN_LIEUTENANT
};
