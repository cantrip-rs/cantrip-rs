use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "Enters with a divinity counter on it if you cast it from your hand" (CR 614.1c, 601.2a): it was cast, and from a hand. A Myojin put onto the
/// battlefield any other way (reanimated, flickered, cast from exile or a graveyard) enters with no counter.
fn enters_with_a_divinity_counter_if_cast_from_your_hand(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if g.obj(entering.object).cast_from == Some(ZoneKind::Hand) {
        entering.counters[CounterKind::Divinity as usize] += 1;
    }
    Ok(())
}

/// "As long as it has a divinity counter on it" (CR 122.1, 604.1): asked live as the layers are worked out.
fn has_a_divinity_counter(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && q.state().obj(source).counters(CounterKind::Divinity) > 0
}

/// "Each opponent discards their hand" (CR 701.9a, 101.4): every opponent's whole hand, all at once, nobody choosing.
fn each_opponent_discards_their_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hands: Vec<ObjectId> = g
        .state()
        .apnap()
        .filter(|&p| p != item.controller)
        .flat_map(|p| g.state().zone(Zone::hand(p)).to_vec())
        .collect();
    g.discard_together(&hands);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_a_divinity_counter_if_cast_from_your_hand),
    statics: &[static_ability(
        Layer::Ability,
        has_a_divinity_counter,
        statics::have_indestructible,
        "This creature has indestructible as long as it has a divinity counter on it.",
    )],
    abilities: &[ability(
        remove_counters(CounterKind::Divinity, 1),
        each_opponent_discards_their_hand,
        "Remove a divinity counter from this creature: Each opponent discards their hand.",
    )],
    ..oracle::MYOJIN_OF_NIGHTS_REACH
};
