use cantrip_api::*;
use cantrip_mechanics::{lands, targets};
use cantrip_oracle as oracle;

/// "Target player gains 2 life, then searches their library for a basic land card, puts it onto the battlefield tapped, then
/// shuffles. Put a +1/+1 counter on up to one target creature." (CR 119.1, 701.23, 122.1): the player searches their own library,
/// whether or not it is the spell's controller's; a target player or creature that is no longer a legal target is left alone
/// (CR 608.2b), the other part still happening. The search ends the first sentence, so the counter goes on after it.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = targets::player(g, item, 0) { gain_life_then_search(g, item.object, player)?; }
    g.end_sentence();
    if let Some(creature) = targets::object(g, item, 1) { g.add_counters(creature, CounterKind::PlusOne, 1); }
    Ok(())
}

/// "Target player gains 2 life, then searches their library for a basic land card, puts it onto the battlefield tapped, then
/// shuffles": the search ends the first sentence's life gain (CR 608.2c).
fn gain_life_then_search(g: &mut dyn Game, source: ObjectId, player: PlayerId) -> Result<(), Suspend> {
    g.gain_life(player, 2);
    g.end_sentence();
    lands::search_basic_land_to_battlefield(g, player, source, true)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER, up_to_one(targets::CREATURE.filter)], effect),
    ..oracle::RESTORATIVE_TECHNIQUE
};
