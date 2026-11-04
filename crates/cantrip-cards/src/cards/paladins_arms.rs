use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::job_select::{HERO, JOB_SELECT};
use cantrip_mechanics::lands;
use cantrip_mechanics::statics::{also_a, grant_delayed_ability, plus_pt};
use cantrip_mechanics::triggers::ward;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        JOB_SELECT,
        // "Lightbringer and Hero's Shield \u{2014} Equip {4}" (CR 702.6a).
        equip(mana("{4}"), "Lightbringer and Hero's Shield \u{2014} Equip {4}"),
    ],
    // The ward the equipped creature has is an ability this Equipment grants (CR 702.21a, 613.1f), so it is the creature's own.
    delayed: &[ward::<1>("Ward {1} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)")],
    statics: &[
        static_ability(Layer::Type, attached_object, also_a::<{ lands::sub(Subtype::Knight) }>, "Equipped creature is a Knight in addition to its other types."),
        static_ability(Layer::PtModify, attached_object, plus_pt::<2, 1>, "Equipped creature gets +2/+1."),
        static_ability(Layer::Ability, attached_object, grant_delayed_ability, "Equipped creature has ward {1}."),
    ],
    ..oracle::PALADINS_ARMS
};

pub static EXTRA: &[&CardDef] = &[&HERO];
