use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add_any_color(1).only_for(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Dinosaur).or_abilities())),
        "{T}: Add one mana of any color. Spend this mana only to cast a Dinosaur spell or activate an ability of a Dinosaur source.",
    )],
    ..oracle::IXALLIS_LOREKEEPER
};
