use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, effects, statics::have_trample, targets};
use cantrip_oracle as oracle;

/// "When this Aura is put into a graveyard from the battlefield" (CR 700.4), looked back on (CR 603.10a). "It" is the
/// card in the graveyard, a new object (CR 400.7), the ability's subject.
fn put_into_graveyard(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, new, from, to, .. } if *old == source && from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard) => {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, put_into_graveyard), effects::return_subject_from_graveyard_to_hand, "When Rancor is put into a graveyard from the battlefield, return it to its owner's hand.")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, cantrip_mechanics::statics::plus_pt::<2, 0>, "Enchanted creature gets +2/+0."),
        static_ability(Layer::Ability, attached_object, have_trample, "Enchanted creature has trample."),
    ],
    ..oracle::RANCOR
};
