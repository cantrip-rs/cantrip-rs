use cantrip_api::*;
use cantrip_mechanics::{effects, warp};
use cantrip_oracle as oracle;

/// "Whenever another creature or artifact you control enters" (CR 603.6a): a permanent other than this one that came under its controller's
/// control as it entered and is a creature or an artifact there - one trigger per permanent, so an artifact creature triggers it once.
fn another_creature_or_artifact_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if *new != me.source
                && to.is(ZoneKind::Battlefield)
                && q.controller(*new) == me.controller
                && {
                    let chars = q.chars(*new);
                    chars.is(CardType::Creature) || chars.is(CardType::Artifact)
                } =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_creature_or_artifact_you_control_enters),
        effects::damage_each_opponent_n::<1>,
        "Whenever another creature or artifact you control enters, this creature deals 1 damage to each opponent.",
    )],
    ways: &[warp::way(mana("{R}"), "Warp {R}")],
    end_step_riders: warp::RIDERS,
    delayed: &[warp::EXILE],
    ..oracle::WEFTSTALKER_ARDENT
};
