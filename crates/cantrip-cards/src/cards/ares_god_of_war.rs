use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::statics::{attacks_each_combat_if_able, itself};
use cantrip_oracle as oracle;

/// "Whenever an attacking creature you control dies" (CR 700.4, 603.10a): it was attacking (CR 506.3) and under its
/// controller's control as it died. The trigger is about the card now in the graveyard, a new object (CR 400.7).
fn attacking_creature_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, new, from, to, .. }
            if from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && q.chars(*old).is(CardType::Creature)
                && q.obj(*old).controller == me.controller
                && q.was_attacking(*old) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, attacking_creature_you_control_dies),
        effects::return_subject_from_graveyard_to_hand,
        "Whenever an attacking creature you control dies, return that card to its owner's hand.",
    )],
    statics: &[static_ability(Layer::Ability, itself, attacks_each_combat_if_able, "Ares attacks each combat if able.")],
    ..oracle::ARES_GOD_OF_WAR
};
