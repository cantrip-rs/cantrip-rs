use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::job_select::{HERO, JOB_SELECT};
use cantrip_mechanics::lands;
use cantrip_mechanics::statics::{also_a, plus_per_artifact_you_control};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        JOB_SELECT,
        // "Machina \u{2014} Equip {4}" (CR 702.6a).
        equip(mana("{4}"), "Machina \u{2014} Equip {4}"),
    ],
    statics: &[
        static_ability(Layer::Type, attached_object, also_a::<{ lands::sub(Subtype::Artificer) }>, "Equipped creature is an Artificer in addition to its other types."),
        static_ability(Layer::PtModify, attached_object, plus_per_artifact_you_control::<2, 2>, "Equipped creature gets +2/+2 for each artifact you control."),
    ],
    ..oracle::MACHINISTS_ARSENAL
};

pub static EXTRA: &[&CardDef] = &[&HERO];
