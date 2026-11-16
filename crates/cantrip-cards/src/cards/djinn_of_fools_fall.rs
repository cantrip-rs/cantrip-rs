use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[plot(mana("{3}{U}"), "Plot {3}{U} (You may pay {3}{U} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)")],
    ..oracle::DJINN_OF_FOOLS_FALL
};
