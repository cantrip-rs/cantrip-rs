use cantrip_api::*;
use cantrip_mechanics::{effects, statics, triggers};
use cantrip_oracle as oracle;

/// "Whenever a creature an opponent controls dies, if its toughness was less than 1" (CR 700.4, 603.4, 603.10a): the
/// creature as it last was on the battlefield, its toughness with every counter and effect on it as it died (a creature
/// with -1/-1 counters, or shrunk until end of turn, dies with a toughness of 0 or less).
fn opponents_creature_dies_with_toughness_below_one(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let fired = triggers::creature_an_opponent_controls_dies(q, me, event)?;
    match event {
        Event::ZoneChange { old, .. } if q.chars(*old).toughness.is_some_and(|toughness| toughness < 1) => Some(fired),
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, statics::your_creatures, statics::have_wither, "Creatures you control have wither.")],
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, opponents_creature_dies_with_toughness_below_one),
        |g, item| effects::draw(g, item, 1),
        "Whenever a creature an opponent controls dies, if its toughness was less than 1, draw a card.",
    )],
    ..oracle::MASSACRE_GIRL_KNOWN_KILLER
};
