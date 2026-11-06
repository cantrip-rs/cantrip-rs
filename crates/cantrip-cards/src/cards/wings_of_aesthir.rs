use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets};
use cantrip_oracle as oracle;

/// The enchanted creature has flying and first strike (CR 604.1, 613.1f).
fn flying_and_first_strike(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    statics::have_flying(q, source, chars);
    statics::have_first_strike(q, source, chars);
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Ability, attach::attached_object, flying_and_first_strike, "Enchanted creature gets +1/+0 and has flying and first strike."),
        static_ability(Layer::PtModify, attach::attached_object, statics::plus_pt_saturating::<1, 0>, "Enchanted creature gets +1/+0 and has flying and first strike.").continues(),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::WINGS_OF_AESTHIR
};
