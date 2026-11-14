use cantrip_api::*;
use cantrip_mechanics::{poison, triggers};
use cantrip_oracle as oracle;

/// "Whenever a creature you control with deathtouch deals combat damage to a player" (CR 510.2, 603.2, 702.2): deathtouch as the
/// layers give it to the creature as it dealt the damage. One trigger for each creature and player, with "that player" fixed as
/// the trigger's subject.
fn creature_you_control_with_deathtouch_deals_combat_damage_to_player(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: from, .. } if q.chars(*from).has(Keyword::Deathtouch) => {
            triggers::creature_you_control_deals_combat_damage_to_player(q, me, event)
        }
        _ => None,
    }
}

/// Deathtouch is a core keyword. "Whenever a creature you control with deathtouch deals combat damage to a player, that player gets
/// two poison counters." (CR 603.2, 510.2, 122.1).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::DAMAGE, creature_you_control_with_deathtouch_deals_combat_damage_to_player),
        poison::give::<2>,
        "Whenever a creature you control with deathtouch deals combat damage to a player, that player gets two poison counters.",
    )],
    ..oracle::FYNN_THE_FANGBEARER
};
