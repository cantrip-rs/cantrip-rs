use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_mechanics::{creature_cards_in_graveyard, tokens};
use cantrip_oracle as oracle;

/// The 2/1 green Varmint creature token Rise of the Varmints makes.
static VARMINT: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(1),
    ..token!(Varmint)
};

/// "Create X 2/1 green Varmint creature tokens, where X is the number of creature cards in your graveyard" (CR 111.1, 608.2h): the cards
/// as the spell resolves, made together as one event (CR 101.4); none for an empty graveyard.
fn create_varmints(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = creature_cards_in_graveyard(g, item.controller) as u32;
    tokens::create(g, item.controller, &VARMINT, x)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], create_varmints),
    abilities: &[plot(
        mana("{2}{G}"),
        "Plot {2}{G} (You may pay {2}{G} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
    )],
    ..oracle::RISE_OF_THE_VARMINTS
};

pub static EXTRA: &[&CardDef] = &[&VARMINT];
