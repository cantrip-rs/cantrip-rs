use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        // "Spend this mana only to cast an Ally spell or activate an ability of an Ally source" (CR 106.6, 205.3).
        mana_ability(
            TAP,
            add_any_color(1).only_for(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Ally).or_abilities())),
            "{T}: Add one mana of any color. Spend this mana only to cast an Ally spell or activate an ability of an Ally source.",
        ),
        ability(cost("{5}, {T}"), tokens::create_an_ally, "{5}, {T}: Create a 1/1 white Ally creature token."),
    ],
    ..oracle::JASMINE_DRAGON_TEA_SHOP
};
