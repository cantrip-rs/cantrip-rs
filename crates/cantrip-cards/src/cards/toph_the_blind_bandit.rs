use cantrip_api::*;
use cantrip_mechanics::{earthbend, triggers};
use cantrip_oracle as oracle;

/// "Toph's power is equal to the number of +1/+1 counters on lands you control" (CR 604.3, 613.4a): a characteristic-defining
/// ability, read live. "You" is Toph's controller; a land is counted by the types it has once the earlier layers have run, and
/// every +1/+1 counter on it counts, whether or not the land is a creature (the earthbent land and any other land alike).
fn power_is_plus_one_counters_on_your_lands(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let you = q.controller(source);
    let state = q.state();
    let counters: i32 = state.battlefield().iter().copied()
        .filter(|&id| q.controller(id) == you && q.chars(id).is(CardType::Land))
        .map(|land| state.obj(land).counters(CounterKind::PlusOne).max(0))
        .sum();
    chars.power = Some(counters);
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, earthbend::effect::<2>, "When Toph enters, earthbend 2.").targets(&[earthbend::TARGET])],
    statics: &[characteristic_defining(
        Layer::PtDefine,
        power_is_plus_one_counters_on_your_lands,
        "Toph's power is equal to the number of +1/+1 counters on lands you control.",
    ).lost_pt(0, 3)],
    delayed: &[earthbend::RETURN],
    ..oracle::TOPH_THE_BLIND_BANDIT
};
