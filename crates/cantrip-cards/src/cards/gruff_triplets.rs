use cantrip_api::*;
use cantrip_mechanics::{copy, effects, library, targets, triggers::{THIS_DIES, THIS_ENTERS}};
use cantrip_oracle as oracle;

/// "When this creature enters, if it isn't a token, create two tokens that are copies of it." (CR 603.4, 707.2, 111.1): the intervening
/// "if" is asked as it triggers and as it resolves, and a token copy entering triggers nothing, so the copies make no more. Both copies
/// are made at once, of Gruff Triplets as it is, or as it last was if it has left the battlefield (CR 608.2h).
fn create_two_copies(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    copy::create_token_copies_of_it(g, item, 2, |_, values| values)
}

/// "When this creature dies, put a number of +1/+1 counters equal to its power on each creature you control named Gruff Triplets."
/// (CR 122.1, 201.2a, 608.2h): the power is the one it had as it last existed on the battlefield (the amount is fixed as the ability
/// resolves, CR 608.2h), put on each creature its controller controls then that has the name, all at once (CR 101.4). A creature with
/// no power puts none (CR 107.1b); the dying creature is in the graveyard by then and gets none.
fn put_its_power_in_counters_on_each_named_creature_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = targets::power_or_last_known(g, item.object);
    if power > 0 {
        let name = g.card_id(&CARD);
        let named: Vec<ObjectId> = g
            .permanents_of(item.controller)
            .into_iter()
            .filter(|&id| g.chars(id).is(CardType::Creature) && library::object_has_name(g, id, name))
            .collect();
        effects::counters_on_each(g, &named, CounterKind::PlusOne, power);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, create_two_copies, "When this creature enters, if it isn't a token, create two tokens that are copies of it.").condition(copy::not_a_token),
        triggered(
            THIS_DIES,
            put_its_power_in_counters_on_each_named_creature_you_control,
            "When this creature dies, put a number of +1/+1 counters equal to its power on each creature you control named Gruff Triplets.",
        ),
    ],
    ..oracle::GRUFF_TRIPLETS
};
