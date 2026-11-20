use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::{creature_you_control_deals_combat_damage_to_player, THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Whenever a legendary creature you control deals combat damage to a player" (CR 510.2, 603.2, 205.4a): `creature_you_control_deals_combat_damage_to_player`
/// for a creature that is legendary, one trigger for each such creature and player.
fn legendary_creature_you_control_deals_combat_damage_to_player(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source, .. } if q.chars(*source).supertypes.legendary => creature_you_control_deals_combat_damage_to_player(q, me, event),
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            effects::counter_on_each_creature_you_control::<{ CounterKind::Deathtouch as usize }>,
            "When Vraska Joins Up enters, put a deathtouch counter on each creature you control.",
        ),
        triggered(
            when_asked(EventMask::DAMAGE, legendary_creature_you_control_deals_combat_damage_to_player),
            |g, item| effects::draw(g, item, 1),
            "Whenever a legendary creature you control deals combat damage to a player, draw a card.",
        ),
    ],
    ..oracle::VRASKA_JOINS_UP
};
