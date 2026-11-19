use cantrip_api::*;
use cantrip_mechanics::statics::{plus_pt, your_creatures};
use cantrip_oracle as oracle;

/// "Enchanted creatures you control" (CR 303.4, 613.1f): `your_creatures` with an Aura attached, whoever controls the Aura (a Role
/// token counts), judged on what the earlier layers made each object. A non-Aura enchantment (this one) is not attached to
/// anything, so it enchants nothing.
fn enchanted_creatures_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    your_creatures(q, source, affected, so_far)
        && q.state().battlefield().iter().any(|&id| q.state().obj(id).attached_to == Some(affected) && q.chars(id).has_subtype(Subtype::Aura))
}

/// "Enchanted creatures you control get +2/+2." (CR 613.4c, layer 7c).
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, enchanted_creatures_you_control, plus_pt::<2, 2>, "Enchanted creatures you control get +2/+2.")],
    ..oracle::A_TALE_FOR_THE_AGES
};
