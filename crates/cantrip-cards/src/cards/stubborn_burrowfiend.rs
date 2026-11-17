use cantrip_api::*;
use cantrip_mechanics::{creature_cards_in_graveyard, pump, saddle};
use cantrip_oracle as oracle;

/// "Whenever this creature becomes saddled for the first time each turn" (CR 702.171b, 603.2): a saddle ability of this Mount resolved,
/// and no earlier resolution this turn had it become saddled, the history being as it was just before this event (the saddle ability
/// again on an already saddled Mount is not the first time). A trigger condition, not a limit on how often the ability triggers.
fn this_becomes_saddled_for_the_first_time_each_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::BecameSaddled { permanent }
            if *permanent == me.source && q.history().count(EventQuery::kind(EventMask::BECAME_SADDLED).object(me.source).this_turn()) == 0 =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

const THIS_BECOMES_SADDLED_FOR_THE_FIRST_TIME_EACH_TURN: Trigger =
    when_asked(EventMask::BECAME_SADDLED, this_becomes_saddled_for_the_first_time_each_turn);

/// "Mill two cards, then this creature gets +X/+X until end of turn, where X is the number of creature cards in your graveyard" (CR 701.17a,
/// 608.2c, 611.2a): the milled cards are in the graveyard before X is counted, as the ability resolves, and the bonus is one
/// continuous effect from the Burrowfiend, nothing once it has left the battlefield.
fn mill_two_then_grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 2);
    g.end_sentence();
    let x = creature_cards_in_graveyard(g, item.controller) as i32;
    pump(g, item.object, item.object, x, x);
    Ok(())
}

/// "Saddle 2" (CR 702.171a) is the shared mechanic.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_BECOMES_SADDLED_FOR_THE_FIRST_TIME_EACH_TURN,
            mill_two_then_grow,
            "Whenever this creature becomes saddled for the first time each turn, mill two cards, then this creature gets +X/+X until end of turn, where X is the number of creature cards in your graveyard.",
        ),
        saddle::ability(2, "Saddle 2 (Tap any number of other creatures you control with total power 2 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::STUBBORN_BURROWFIEND
};
