use cantrip_api::*;
use cantrip_mechanics::{lands, statics};
use cantrip_oracle as oracle;

/// Other Kobold creatures you control, with Kobold read from this object's text (CR 612.2).
fn other_kobolds(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    chars.is(CardType::Creature) && statics::other_of_subtype_you_control::<{ lands::sub(Subtype::Kobold) }>(q, source, affected, chars)
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, other_kobolds, statics::have_first_strike, "Other Kobold creatures you control have first strike.")],
    ..oracle::KOBOLD_OVERLORD
};
