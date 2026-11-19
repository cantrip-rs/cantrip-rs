use cantrip_api::*;
use cantrip_mechanics::statics::{base_pt, have_flying, other_creatures_you_control};
use cantrip_oracle as oracle;

/// "Other creatures you control that are enchanted by Auras you control" (CR 303.4, 613.1f): `other_creatures_you_control` with an
/// Aura attached that the Archon's controller controls, so a creature wearing only an opponent's Aura is out. Judged on what the
/// earlier layers made each object.
fn other_creatures_you_control_enchanted_by_your_auras(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    other_creatures_you_control(q, source, affected, so_far)
        && q.state().battlefield().iter().any(|&id| q.state().obj(id).attached_to == Some(affected) && q.chars(id).has_subtype(Subtype::Aura) && q.controller(id) == q.controller(source))
}

/// "... have base power and toughness 4/4 and have flying." (CR 613.4b, layer 7b, and layer 6 for flying): the 4/4 replaces the
/// creature's printed values and the counters, pumps and other layer 7c changes still apply on top of it.
pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Ability, other_creatures_you_control_enchanted_by_your_auras, have_flying, "Other creatures you control that are enchanted by Auras you control have base power and toughness 4/4 and have flying."),
        static_ability(Layer::PtSet, other_creatures_you_control_enchanted_by_your_auras, base_pt::<4, 4>, "Other creatures you control that are enchanted by Auras you control have base power and toughness 4/4 and have flying.").continues(),
    ],
    ..oracle::ARCHON_OF_THE_WILD_ROSE
};
