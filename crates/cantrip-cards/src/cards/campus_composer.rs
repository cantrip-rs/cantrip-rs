use cantrip_api::*;
use cantrip_mechanics::{prepare, tokens, triggers::ward};
use cantrip_oracle as oracle;

/// "Create a 3/3 blue and red Elemental creature token with flying."
fn aqueous_aria_effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ELEMENTAL_3_3_BLUE_RED, 1)?;
    Ok(())
}

/// Campus Composer's prepare spell (CR 722): not a card here, only ever a copy.
pub static AQUEOUS_ARIA: CardDef = CardDef {
    name: "Aqueous Aria",
    cost: mana("{4}{U}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], aqueous_aria_effect),
    text: "Create a 3/3 blue and red Elemental creature token with flying.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[ward::<2>("Ward {2}")],
    enters: Some(prepare::enters_prepared),
    prepare: Some(&AQUEOUS_ARIA),
    ..oracle::CAMPUS_COMPOSER
};

pub static EXTRA: &[&CardDef] = &[&AQUEOUS_ARIA, &tokens::ELEMENTAL_3_3_BLUE_RED];
