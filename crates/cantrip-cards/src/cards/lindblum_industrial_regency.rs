use cantrip_api::*;
use cantrip_mechanics::triggers::NONCREATURE_SPELL_YOU_CAST;
use cantrip_mechanics::{effects, mana, tokens};
use cantrip_oracle as oracle;

/// "Whenever you cast a noncreature spell, this token deals 1 damage to each opponent."
fn ping(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent(g, item, 1);
    Ok(())
}

/// The 0/1 black Wizard creature token with "Whenever you cast a noncreature spell, this token deals 1 damage to each
/// opponent."
pub static WIZARD: CardDef = CardDef {
    name: "Wizard Token",
    token_name: Some(&[Subtype::Wizard]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Wizard]),
    power: Some(0),
    toughness: Some(1),
    abilities: &[triggered(NONCREATURE_SPELL_YOU_CAST, ping, "Whenever you cast a noncreature spell, this token deals 1 damage to each opponent.")],
    text: "Whenever you cast a noncreature spell, this token deals 1 damage to each opponent.",
    ..CardDef::BLANK
};

/// Mage Siege: "Create a 0/1 black Wizard creature token with ..." (CR 111.1).
fn mage_siege(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &WIZARD, 1)?;
    Ok(())
}

/// The Adventure (CR 715): an instant, cast from the hand whenever an instant could be, then exiled on its adventure, from
/// where the land may be played (CR 715.3d).
pub static MAGE_SIEGE: CardDef = CardDef { spell: spell(&[], mage_siege), ..oracle::MAGE_SIEGE };

pub static CARD: CardDef = CardDef {
    // "This land enters tapped" (CR 614.1c).
    enters: Some(mana::enters_tapped),
    abilities: &[mana_ability(TAP, add(Mana::R), "{T}: Add {R}.")],
    adventure: Some(&MAGE_SIEGE),
    ..oracle::LINDBLUM_INDUSTRIAL_REGENCY
};

pub static EXTRA: &[&CardDef] = &[&MAGE_SIEGE, &WIZARD];
