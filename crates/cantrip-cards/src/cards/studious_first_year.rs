use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

/// "Search your library for a basic land card, put it onto the battlefield tapped, then shuffle" (CR 701.23).
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lands::search_basic_land_to_battlefield(g, item.controller, item.object, true)
}

/// Studious First-Year's prepare spell (CR 722): not a card, only ever a copy.
pub static RAMPANT_GROWTH: CardDef = CardDef {
    name: "Rampant Growth",
    cost: mana("{1}{G}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], search),
    text: "Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(cantrip_mechanics::prepare::enters_prepared),
    prepare: Some(&RAMPANT_GROWTH),
    ..oracle::STUDIOUS_FIRST_YEAR
};

pub static EXTRA: &[&CardDef] = &[&RAMPANT_GROWTH];
