use cantrip_api::*;
use cantrip_mechanics::{card_types, untapped_you_control};
use cantrip_oracle as oracle;

/// "As long as this creature has three or more counters on it" (CR 122.1): counters of any kind, added up.
fn has_three_or_more_counters(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && q.state().obj(source).counter_array().iter().map(|&n| n.max(0) as i32).sum::<i32>() >= 3
}

/// "Put a +1/+1 counter on this creature. Scry 1." The counter goes only on this creature while it is still on the
/// battlefield (CR 400.7); the scry happens either way.
fn grow_and_scry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    cantrip_mechanics::scry(g, item.controller, item.object, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        tap_untapped(3, untapped_you_control::<{ card_types(&[CardType::Artifact, CardType::Creature]) }>),
        grow_and_scry,
        "Tap three untapped artifacts and/or creatures you control: Put a +1/+1 counter on this creature. Scry 1. Activate only as a sorcery.",
    )
    .sorcery_speed()],
    statics: &[static_ability(
        Layer::Ability,
        has_three_or_more_counters,
        cantrip_mechanics::statics::have_flying_and_vigilance,
        "As long as this creature has three or more counters on it, it has flying and vigilance.",
    )],
    ..oracle::WARDEN_OF_THE_INNER_SKY
};
