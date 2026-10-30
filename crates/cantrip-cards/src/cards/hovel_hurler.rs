use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets};
use cantrip_oracle as oracle;

/// "Another target creature you control gets +1/+0 and gains flying until end of turn."
fn lend_wings(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 1, 0);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Flying);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_minus_one_counters::<2>),
    abilities: &[ability(
        cost("{R/W}{R/W}").and(REMOVE_A_COUNTER),
        lend_wings,
        "{R/W}{R/W}, Remove a counter from this creature: Another target creature you control gets +1/+0 and gains flying until end of turn. Activate only as a sorcery.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])
    .sorcery_speed()],
    ..oracle::HOVEL_HURLER
};
