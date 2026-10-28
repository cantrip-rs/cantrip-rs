use cantrip_api::*;
use cantrip_mechanics::history::drawn_this_turn;
use cantrip_mechanics::{effects, pump, targets, triggers};
use cantrip_oracle as oracle;

/// The intervening "if you've drawn two or more cards this turn" (CR 603.4): asked as it triggers and as it resolves.
fn drawn_two_or_more(q: &dyn Query, asker: &Asker) -> bool {
    drawn_this_turn(q, asker.controller) >= 2
}

/// "Another target creature you control gets +3/+0 and gains first strike until end of turn": one instruction (CR 608.2c).
/// Nothing happens if it is no longer a legal target (CR 608.2b).
fn boost(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 3, 0);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        boost,
        "At the beginning of combat on your turn, if you've drawn two or more cards this turn, another target creature you control gets +3/+0 and gains first strike until end of turn.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])
    .condition(drawn_two_or_more)],
    ..oracle::LAKE_TOWN_TOYMAKER
};
