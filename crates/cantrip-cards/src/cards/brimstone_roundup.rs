use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_mechanics::{flurry, tokens};
use cantrip_oracle as oracle;

/// "Whenever you cast your second spell each turn, create a 1/1 red Mercenary creature token ..." (CR 603.2, 111.1): the second
/// spell its controller has cast this turn, counted by the cast events of the turn, so spells cast before this entered count.
pub static CARD: CardDef = CardDef {
    abilities: &[
        flurry::trigger(
            "Whenever you cast your second spell each turn, create a 1/1 red Mercenary creature token with \"{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.\"",
            tokens::create_a_mercenary,
        ),
        plot(
            mana("{2}{R}"),
            "Plot {2}{R} (You may pay {2}{R} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
        ),
    ],
    ..oracle::BRIMSTONE_ROUNDUP
};

pub static EXTRA: &[&CardDef] = &[&tokens::MERCENARY];
