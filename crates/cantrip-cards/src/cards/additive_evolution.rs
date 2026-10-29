use cantrip_api::*;
use cantrip_mechanics::triggers::{BEGINNING_OF_YOUR_COMBAT, THIS_ENTERS};
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "When this enchantment enters, create a 0/0 green and blue Fractal creature token. Put three +1/+1 counters on it."
fn fractal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for token in tokens::create(g, item.controller, &tokens::FRACTAL, 1)? {
        g.add_counters(token, CounterKind::PlusOne, 3);
    }
    Ok(())
}

/// "At the beginning of combat on your turn, put a +1/+1 counter on target creature you control. It gains vigilance until end of
/// turn." Two sentences, so the counter and the keyword are separate instructions (CR 608.2c); nothing if the target is no longer
/// legal (CR 608.2b).
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    g.end_sentence();
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Vigilance);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, fractal, "When this enchantment enters, create a 0/0 green and blue Fractal creature token. Put three +1/+1 counters on it."),
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            grow,
            "At the beginning of combat on your turn, put a +1/+1 counter on target creature you control. It gains vigilance until end of turn.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    ..oracle::ADDITIVE_EVOLUTION
};

pub static EXTRA: &[&CardDef] = &[&tokens::FRACTAL];
