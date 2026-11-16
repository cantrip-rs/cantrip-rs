use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "Exile any number of creatures you control, then return them to the battlefield under their owner's control. Then repeat
/// this process X more times." (CR 608.2c, 400.7, 111.7): the process runs once and then X more times, one pass at a time. Each
/// pass chooses afresh among the creatures its controller has then (asked even when there is only one, and when there is none
/// nothing is exiled), exiles the ones chosen together and returns them together as new objects (a token ceases to exist), so
/// the next pass sees only the new objects. "Any number" may be none, which is a pass that does nothing.
fn blink_any_number_of_creatures_and_repeat_x_more_times(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for _ in 0..=item.x {
        let creatures: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
        let chosen = choose::select(g, item.controller, item.object, Purpose::Exile, creatures, select::creature_permanents(select::ANY_NUMBER))?;
        cantrip_mechanics::exile_and_return(g, &chosen)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], blink_any_number_of_creatures_and_repeat_x_more_times), ..oracle::ANOTHER_ROUND };

pub static EXTRA: &[&CardDef] = &[];
