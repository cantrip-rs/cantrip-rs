use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on each other creature you control that's a token or a Rabbit. Then if creatures you control have total
/// power 10 or greater, draw a card" (CR 608.2, 122.1, 205.3, 111.1): the counters go on the creatures as it resolves, all at
/// once (CR 101.4); "then" reads the total power after them (CR 608.2c). The Rabbit subtype is read through the words of the
/// source's text (CR 612.2).
fn counters_on_other_tokens_and_rabbits_then_draw_at_ten_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rabbit = g.words(item.object).subtype(Subtype::Rabbit);
    let creatures: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| id != item.object && g.chars(id).is(CardType::Creature) && (g.obj(id).token || g.chars(id).has_subtype(rabbit)))
        .collect();
    effects::counters_on_each(g, &creatures, CounterKind::PlusOne, 1);
    if cantrip_mechanics::creatures_have_total_power_at_least(g, item.controller, 10) {
        effects::draw(g, item, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        counters_on_other_tokens_and_rabbits_then_draw_at_ten_power,
        "Whenever Finneas attacks, put a +1/+1 counter on each other creature you control that's a token or a Rabbit. Then if creatures you control have total power 10 or greater, draw a card.",
    )],
    ..oracle::FINNEAS_ACE_ARCHER
};
