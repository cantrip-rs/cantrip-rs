use cantrip_api::*;
use cantrip_mechanics::statics::{have_deathtouch, plus_pt};
use cantrip_mechanics::{lands, statics};
use cantrip_oracle as oracle;

/// "Skeletons you control and other Zombies you control" (CR 613.1f, 205.3m): a Skeleton under the source's controller's control, the
/// Baron itself among them if it were one, or another of their Zombies, the types as the earlier layers made them.
fn skeletons_and_other_zombies_you_control(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    statics::your_of_subtype::<{ lands::sub(Subtype::Skeleton) }>(view, source, affected, so_far)
        || statics::other_of_subtype_you_control::<{ lands::sub(Subtype::Zombie) }>(view, source, affected, so_far)
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::PtModify, skeletons_and_other_zombies_you_control, plus_pt::<1, 1>, "Skeletons you control and other Zombies you control get +1/+1 and have deathtouch."),
        static_ability(Layer::Ability, skeletons_and_other_zombies_you_control, have_deathtouch, "Skeletons you control and other Zombies you control get +1/+1 and have deathtouch."),
    ],
    ..oracle::DEATH_BARON
};
