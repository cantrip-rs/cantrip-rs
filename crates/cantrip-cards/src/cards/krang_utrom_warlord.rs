use cantrip_api::*;
use cantrip_oracle as oracle;

const GRANTED: Keywords = Keywords::of(&[Keyword::Flying, Keyword::Trample, Keyword::Indestructible, Keyword::Haste]);

fn apply(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = Keywords(chars.keywords.0 | GRANTED.0);
}

/// "Other artifact creatures you control have flying, trample, indestructible, and haste."
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, cantrip_mechanics::statics::other_artifact_creatures_you_control, apply, "Other artifact creatures you control have flying, trample, indestructible, and haste.")],
    ..oracle::KRANG_UTROM_WARLORD
};
