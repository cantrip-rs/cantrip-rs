use cantrip_api::*;
use cantrip_mechanics::{disguise, effects};
use cantrip_oracle as oracle;

/// "Whenever this creature or another creature you control is turned face up" (CR 116.2b, 708.8, 603.2): one trigger for each creature
/// turned face up while the source's controller controls it, whoever turned it up, about that creature. A face-down Performer has no
/// abilities, so it triggers only from the turning up of itself.
fn creature_you_control_turned_face_up(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::TurnedFaceUp { permanent, .. } if q.controller(*permanent) == q.controller(source) && q.chars(*permanent).is(CardType::Creature) => {
            Fired::about(Target::Object(*permanent))
        }
        _ => None,
    }
}

/// Disguise {R} (CR 702.168). "That creature deals damage equal to its power to each opponent" (CR 120.3): the creature turned up is the
/// source, at the power it has as the ability resolves (as it last had it if it has left, CR 608.2h).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::TURNED_FACE_UP, creature_you_control_turned_face_up),
        effects::subject_deals_damage_equal_to_its_power_to_each_opponent,
        "Whenever this creature or another creature you control is turned face up, that creature deals damage equal to its power to each opponent.",
    )],
    ways: &[disguise::way(Cost::mana(mana("{R}")), "Disguise {R}")],
    ..oracle::PYROTECHNIC_PERFORMER
};
