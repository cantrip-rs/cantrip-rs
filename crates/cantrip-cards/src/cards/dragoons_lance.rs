use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::job_select::{HERO, JOB_SELECT};
use cantrip_mechanics::statics::{also_a, have_flying, plus_pt};
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

/// "During your turn, equipped creature has flying" (CR 604.1, 613.1f): the creature the Equipment is attached to, only while
/// the Equipment's controller is the active player (CR 102.1), asked again whenever characteristics are computed.
fn equipped_during_your_turn(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    q.state().turn.active == q.state().obj(source).controller && attached_object(q, source, affected, so_far)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        JOB_SELECT,
        // "Gae Bolg \u{2014} Equip {4}" (CR 702.6a).
        equip(mana("{4}"), "Gae Bolg \u{2014} Equip {4}"),
    ],
    statics: &[
        static_ability(Layer::Type, attached_object, also_a::<{ lands::sub(Subtype::Knight) }>, "Equipped creature is a Knight in addition to its other types."),
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 0>, "Equipped creature gets +1/+0."),
        static_ability(Layer::Ability, equipped_during_your_turn, have_flying, "During your turn, equipped creature has flying."),
    ],
    ..oracle::DRAGOONS_LANCE
};

pub static EXTRA: &[&CardDef] = &[&HERO];
