use cantrip_api::*;
use cantrip_mechanics::{effects::ping_subject_controller, statics::players_cant_gain_life};
use cantrip_oracle as oracle;

/// "Whenever another creature enters" (CR 603.6a): a creature other than this one, under any player's control, the ones
/// entering together with it included (CR 603.6a, the rulings). The entering creature is the trigger's subject.
fn another_creature_enters(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if *new != source && to.is(ZoneKind::Battlefield) && q.chars(*new).is(CardType::Creature) => {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    cant_gain_life: &[players_cant_gain_life],
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, another_creature_enters), ping_subject_controller, "Whenever another creature enters, this creature deals 1 damage to that creature's controller.")],
    ..oracle::RAMPAGING_FEROCIDON
};
