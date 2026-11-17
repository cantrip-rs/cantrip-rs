use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, |g, item| effects::gain_life(g, item, 3), "When this creature enters, you gain 3 life."),
        plot(
            mana("{3}{G}"),
            "Plot {3}{G} (You may pay {3}{G} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
        ),
    ],
    ..oracle::SPINEWOODS_PALADIN
};
