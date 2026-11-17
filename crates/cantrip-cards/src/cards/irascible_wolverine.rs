use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_mechanics::{library, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            library::exile_top_card_play_this_turn,
            "When this creature enters, exile the top card of your library. Until end of turn, you may play that card.",
        ),
        plot(
            mana("{2}{R}"),
            "Plot {2}{R} (You may pay {2}{R} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
        ),
    ],
    ..oracle::IRASCIBLE_WOLVERINE
};
