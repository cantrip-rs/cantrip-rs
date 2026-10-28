use cantrip_api::*;
use cantrip_mechanics::triggers::{CREATURE_SPELL_YOU_CAST, LANDFALL};
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Put X +1/+1 counters on target creature you control, where X is that spell's mana value": the spell's as the ability
/// resolves, or as it last was on the stack if it has left (CR 608.2h: the ability still resolves if the spell is countered), an
/// {X} in its cost counting as the X announced (CR 202.3e). A value past what a counter count holds (RX-5) puts that many.
fn put_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let Target::Object(spell) = item.subject else { return Ok(()) };
    let x = g.mana_value(spell).min(i16::MAX as u32) as i16;
    g.add_counters(creature, CounterKind::PlusOne, x);
    Ok(())
}

/// "Create a 2/2 green Bear creature token." (CR 111.1, 701.7a)
fn make_a_bear(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::BEAR, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            CREATURE_SPELL_YOU_CAST,
            put_counters,
            "Whenever you cast a creature spell, put X +1/+1 counters on target creature you control, where X is that spell's mana value.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
        triggered(LANDFALL, make_a_bear, "Landfall \u{2014} Whenever a land you control enters, create a 2/2 green Bear creature token."),
    ],
    ..oracle::DANCING_FROM_DARK_TO_DAWN
};

pub static EXTRA: &[&CardDef] = &[&tokens::BEAR];
