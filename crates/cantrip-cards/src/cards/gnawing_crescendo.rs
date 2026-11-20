use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, tokens, triggers};
use cantrip_oracle as oracle;

/// "Creatures you control get +2/+0 until end of turn. Whenever a nontoken creature you control dies this turn, create a 1/1 black
/// Rat creature token with 'This token can't block.'" (CR 611.2c, 603.7): the pump reaches the creatures there are as it resolves;
/// the delayed ability is made as the spell resolves and watches the rest of the turn, once for each nontoken creature that dies.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::creatures_you_control_get::<2, 0>(g, item)?;
    g.end_sentence();
    delayed::this_turn(g, item, item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    delayed: &[triggered(
        triggers::NONTOKEN_CREATURE_YOU_CONTROL_DIES,
        tokens::create_rats_that_cant_block::<1>,
        "Whenever a nontoken creature you control dies this turn, create a 1/1 black Rat creature token with \"This token can't block.\"",
    )],
    ..oracle::GNAWING_CRESCENDO
};

pub static EXTRA: &[&CardDef] = &[&tokens::RAT_CANT_BLOCK];
