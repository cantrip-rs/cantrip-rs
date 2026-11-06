use cantrip_api::*;
use cantrip_mechanics::{pump, targets};
use cantrip_oracle as oracle;

/// The current or last known enchanted creature gets +2/+2 until end of turn (CR 303.4m, 608.2h, 611.2a).
fn enchanted_creature_gets_2_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = g.obj(item.object).attached_to {
        pump(g, item.object, creature, 2, 2);
    }
    Ok(())
}

/// The restriction applies to this object regardless of its controller (CR 602.5b).
fn enchanted_creature_gets_2_2_each_turn(q: &dyn Query, asker: &Asker) -> bool {
    let this = AbilityRef {
        card: {
            let o = q.state().obj(asker.source);
            o.copied_card().unwrap_or(o.card)
        },
        index: 0,
    };
    (q
        .history()
        .this_turn()
        .iter()
        .filter(|r| {
            matches!(
                r.event, Event::AbilityActivated { source, ability, .. } if source ==
                asker.source && ability == this
            )
        })
        .count() as i64) < 3
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{B}").and(life(1)),
                enchanted_creature_gets_2_2,
                "{B}, Pay 1 life: Enchanted creature gets +2/+2 until end of turn. Activate no more than three times each turn.",
            )
            .condition(enchanted_creature_gets_2_2_each_turn),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::SOUL_KISS
};
