use cantrip_api::*;
use cantrip_mechanics::{pump, targets};
use cantrip_oracle as oracle;

/// Target an Orc creature, reading the creature type through the source's text (CR 115.1c, 612.2).
fn orc_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t)
        && targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Orc])
}

/// Flip on resolution, then give the target the corresponding bonus until end of turn (CR 705.1, 611.2a).
fn flip(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let won = g.flip_coin(item.controller);
    g.end_sentence();
    if won {
        pump(g, item.object, creature, 2, 0);
    } else {
        pump(g, item.object, creature, 0, -2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}"),
            flip,
            "{1}: Flip a coin. If you win the flip, target Orc creature gets +2/+0 until end of turn. If you lose the flip, it gets -0/-2 until end of turn.",
        )
        .targets(&[target(orc_creature)]),
    ],
    ..oracle::ORCISH_CAPTAIN
};
