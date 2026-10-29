use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Create two 2/2 red and white Spirit creature tokens."
fn spirits(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::SPIRIT_2_2_RED_WHITE, 2)?;
    Ok(())
}

/// Strife Scholar's prepare spell (CR 722): not a card here, only ever a copy.
pub static AWAKEN_THE_AGES: CardDef = CardDef {
    name: "Awaken the Ages",
    cost: mana("{5}{R}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], spirits),
    text: "Create two 2/2 red and white Spirit creature tokens.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggers::ward_life::<2>("Ward\u{2014}Pay 2 life.")],
    enters: Some(cantrip_mechanics::prepare::enters_prepared),
    prepare: Some(&AWAKEN_THE_AGES),
    ..oracle::STRIFE_SCHOLAR
};

pub static EXTRA: &[&CardDef] = &[&AWAKEN_THE_AGES, &tokens::SPIRIT_2_2_RED_WHITE];
