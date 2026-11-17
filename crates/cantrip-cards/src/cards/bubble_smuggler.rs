use cantrip_api::*;
use cantrip_mechanics::disguise;
use cantrip_oracle as oracle;

/// "As this creature is turned face up, put four +1/+1 counters on it." (CR 708.11): applied by the engine as the permanent turns
/// face up, however it does (the special action, or an effect), before it is face up for anything to see.
fn four_plus_one_counters(g: &mut dyn Game, asker: &Asker) {
    g.add_counters(asker.source, CounterKind::PlusOne, 4);
}

pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{5}{U}")), "Disguise {5}{U}")],
    as_turned_face_up: Some(four_plus_one_counters),
    ..oracle::BUBBLE_SMUGGLER
};
