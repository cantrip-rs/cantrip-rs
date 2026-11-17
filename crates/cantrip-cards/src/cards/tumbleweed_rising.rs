use cantrip_api::*;
use cantrip_mechanics::{greatest_power, tokens};
use cantrip_mechanics::plot::plot;
use cantrip_oracle as oracle;

/// "Create an X/X green Elemental creature token, where X is the greatest power among creatures you control" (CR 111.3, 608.2h): the
/// creatures as the spell resolves; the token is X/X as it is made (CR 111.4, 707.2), never a 0/0 for a moment, and with no creature
/// (or none above 0 power) X is 0 (CR 107.1b) and the 0/0 token goes to the graveyard (CR 704.5f).
fn create_an_elemental(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let x = greatest_power::among_creatures_you_control(g, you) as i32;
    let values = Copiable::of(g.card_id(&tokens::ELEMENTAL_X_X_GREEN)).sized(x, x);
    g.create_token_copies_of(values, you, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], create_an_elemental),
    abilities: &[plot(
        mana("{2}{G}"),
        "Plot {2}{G} (You may pay {2}{G} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
    )],
    ..oracle::TUMBLEWEED_RISING
};

pub static EXTRA: &[&CardDef] = &[&tokens::ELEMENTAL_X_X_GREEN];
