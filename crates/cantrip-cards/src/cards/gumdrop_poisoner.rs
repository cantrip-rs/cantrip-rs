use cantrip_api::*;
use cantrip_mechanics::{pump, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, up to one target creature gets -X/-X until end of turn, where X is the amount of life you gained this
/// turn." (CR 119.3, 608.2h, 611.2a): X is read from the event history as the ability resolves, the life gained this turn and all of
/// it, however many events it came in, and not reduced by life lost (the ruling). A target no longer legal gets nothing (CR 608.2b).
fn shrink_by_life_gained(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = g.history().life_gained(item.controller) as i32;
    pump(g, item.object, creature, -x, -x);
    Ok(())
}

/// The Adventure (CR 715): Create a Food token.
pub static TEMPT_WITH_TREATS: CardDef = CardDef { spell: spell(&[], tokens::create_a_food_token), ..oracle::TEMPT_WITH_TREATS };

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        shrink_by_life_gained,
        "When this creature enters, up to one target creature gets -X/-X until end of turn, where X is the amount of life you gained this turn.",
    )
    .targets(&[targets::CREATURE.optional()])],
    adventure: Some(&TEMPT_WITH_TREATS),
    ..oracle::GUMDROP_POISONER
};

pub static EXTRA: &[&CardDef] = &[&TEMPT_WITH_TREATS, &tokens::FOOD];
