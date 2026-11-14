use cantrip_api::*;
use cantrip_mechanics::statics::{other_creatures_you_control, plus_pt};
use cantrip_oracle as oracle;

/// "Other creatures you control with flying" (CR 613.1f, layer 6 before 7c): `other_creatures_you_control` that have flying as
/// the earlier layers made them, so a creature that gains flying in layer 6 is pumped (and one that loses it is not), and the
/// Eagle's own flying never pumps itself.
fn other_fliers_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    other_creatures_you_control(q, source, affected, so_far) && so_far.keywords.has(Keyword::Flying)
}

/// Flying is a core keyword read from its characteristics.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, other_fliers_you_control, plus_pt::<1, 1>, "Other creatures you control with flying get +1/+1.")],
    ..oracle::EMPYREAN_EAGLE
};
