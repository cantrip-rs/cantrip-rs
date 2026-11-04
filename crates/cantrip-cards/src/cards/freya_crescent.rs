use cantrip_api::*;
use cantrip_mechanics::statics::{have_flying, itself_during_your_turn};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add(Mana::R).only_for(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Equipment).or_equip_abilities())),
        "{T}: Add {R}. Spend this mana only to cast an Equipment spell or activate an equip ability.",
    )],
    statics: &[static_ability(Layer::Ability, itself_during_your_turn, have_flying, "Jump \u{2014} During your turn, Freya Crescent has flying.")],
    ..oracle::FREYA_CRESCENT
};
