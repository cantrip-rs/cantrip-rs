use cantrip_api::*;
use cantrip_mechanics::{attach, targets};
use cantrip_oracle as oracle;

/// "Enchanted creature has banding." (CR 604.1).
fn enchanted_creature_has_banding_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.keywords = chars.keywords.with(Keyword::Banding);
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            attach::attached_object,
            enchanted_creature_has_banding_apply_0,
            "Enchanted creature has banding.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::COOPERATION
};
