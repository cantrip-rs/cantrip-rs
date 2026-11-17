use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ATTACKS_WHILE_SADDLED, THIS_ENTERS};
use cantrip_mechanics::{choose, delayed, effects, saddle, select};
use cantrip_oracle as oracle;

/// The creatures that saddled `mount` this turn (CR 702.171a: the creatures tapped for the cost), each once, that are still on the
/// battlefield (the choice then only offers creatures): asked of the history (`History::saddled`), so one that has since untapped
/// still counts.
fn creatures_that_saddled_this_turn(q: &dyn Query, mount: ObjectId) -> Vec<ObjectId> {
    let mut saddlers: Vec<ObjectId> = Vec::new();
    for creature in q.history().saddled(mount) {
        if q.is_on_battlefield(creature) && !saddlers.contains(&creature) {
            saddlers.push(creature);
        }
    }
    saddlers
}

/// "Exile it and up to one creature that saddled it this turn, then return those cards to the battlefield under their owner's
/// control." (CR 608.2c, 400.7, 111.7) Fortune, if it is still the permanent that attacked, and the creature its controller
/// chooses (asked even when only one saddled it, and when none did) are exiled together and returned together as new objects.
/// Fortune having left, the chosen creature is still exiled and returned: the instruction is the rest of the sentence.
fn exile_it_and_a_saddler_then_return_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(fortune) = item.subject else { return Ok(()) };
    let saddlers = creatures_that_saddled_this_turn(g, fortune);
    let chosen = choose::select(g, item.controller, item.object, Purpose::Exile, saddlers, select::creature_permanents(select::up_to(1)))?;
    let mut exiled: Vec<ObjectId> = Vec::new();
    if g.is_on_battlefield(fortune) {
        exiled.push(fortune);
    }
    exiled.extend(chosen);
    cantrip_mechanics::exile_and_return(g, &exiled)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::scry_two, "When Fortune enters, scry 2."),
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            delayed::at_end_of_combat_about_this,
            "Whenever Fortune attacks while saddled, at end of combat, exile it and up to one creature that saddled it this turn, then return those cards to the battlefield under their owner's control.",
        ),
        saddle::ability(1, "Saddle 1"),
    ],
    delayed: &[delayed_trigger(exile_it_and_a_saddler_then_return_them, "Exile it and up to one creature that saddled it this turn, then return those cards to the battlefield under their owner's control.")],
    ..oracle::FORTUNE_LOYAL_STEED
};

pub static EXTRA: &[&CardDef] = &[];
