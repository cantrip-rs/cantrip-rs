use cantrip_api::*;
use cantrip_mechanics::{attach, targets};
use cantrip_oracle as oracle;

/// "Enchanted creature has fear." (CR 604.1).
fn enchanted_creature_has_fear_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.keywords = chars.keywords.with(Keyword::Fear);
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            attach::attached_object,
            enchanted_creature_has_fear_apply_0,
            "Enchanted creature has fear.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::FEAR
};
