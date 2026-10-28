use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever Agent Maria Hill becomes tapped to pay a teamwork cost" (CR 702.194a, 603.2): she was one of the creatures tapped for
/// a spell's teamwork cost as it was cast (`Event::TappedForTeamwork`). A tap for mana, for convoke or for any other cost is none,
/// and another creature tapped for the same cost does not trigger her.
fn this_is_tapped_to_pay_a_teamwork_cost(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::TappedForTeamwork { creature, .. } if *creature == source => Fired::PLAIN,
        _ => None,
    }
}

/// "Put a +1/+1 counter on her and draw a card." (CR 122.1, 121.1): both, in one sentence; the counter only if she is still the
/// object that was tapped (CR 400.7), the card whatever became of her.
fn counter_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::TAPPED_FOR_TEAMWORK, this_is_tapped_to_pay_a_teamwork_cost),
        counter_and_draw,
        "Whenever Agent Maria Hill becomes tapped to pay a teamwork cost, put a +1/+1 counter on her and draw a card.",
    )],
    ..oracle::AGENT_MARIA_HILL
};
