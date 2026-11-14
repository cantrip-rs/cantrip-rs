use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, attached_object_during_your_turn, equip};
use cantrip_mechanics::job_select::JOB_SELECT;
use cantrip_mechanics::tokens::HERO_1_1_COLORLESS;
use cantrip_mechanics::statics::{also_a, have_flying, plus_pt};
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        JOB_SELECT,
        // "Gae Bolg \u{2014} Equip {4}" (CR 702.6a).
        equip(mana("{4}"), "Gae Bolg \u{2014} Equip {4}"),
    ],
    statics: &[
        static_ability(Layer::Type, attached_object, also_a::<{ lands::sub(Subtype::Knight) }>, "Equipped creature is a Knight in addition to its other types."),
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 0>, "Equipped creature gets +1/+0."),
        static_ability(Layer::Ability, attached_object_during_your_turn, have_flying, "During your turn, equipped creature has flying."),
    ],
    ..oracle::DRAGOONS_LANCE
};

pub static EXTRA: &[&CardDef] = &[&HERO_1_1_COLORLESS];
