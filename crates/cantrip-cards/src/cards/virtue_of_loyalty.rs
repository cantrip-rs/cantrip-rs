use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on each creature you control. Untap those creatures." The creatures are the ones its controller has as
/// the ability resolves, fixed by the first sentence (CR 608.2c, 608.2h): the counters go on all at once, and then exactly
/// those creatures untap, together.
fn counter_and_untap_each_creature_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = effects::counters_on_creatures_you_control(g, item.controller, CounterKind::PlusOne, 1);
    g.untap_together(&creatures);
    Ok(())
}

pub static ARDENVALE_FEALTY: CardDef = CardDef {
    spell: spell(&[], |g, item| tokens::create(g, item.controller, &tokens::KNIGHT, 1).map(|_| ())),
    ..oracle::ARDENVALE_FEALTY
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        counter_and_untap_each_creature_you_control,
        "At the beginning of your end step, put a +1/+1 counter on each creature you control. Untap those creatures.",
    )],
    adventure: Some(&ARDENVALE_FEALTY),
    ..oracle::VIRTUE_OF_LOYALTY
};

pub static EXTRA: &[&CardDef] = &[&ARDENVALE_FEALTY, &tokens::KNIGHT];
