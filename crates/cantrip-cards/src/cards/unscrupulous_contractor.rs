use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "You may sacrifice a creature. When you do, target player draws two cards and loses 2 life." is the shared
/// `effects::may_sacrifice_a_creature_when_you_do` (CR 603.12): the creature, which may be Unscrupulous Contractor itself, is
/// chosen as the ability resolves, and the reflexive trigger's player is chosen as it goes on the stack. "Plot {2}{B}"
/// (CR 702.170) is the shared mechanic.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            effects::may_sacrifice_a_creature_when_you_do,
            "When this creature enters, you may sacrifice a creature. When you do, target player draws two cards and loses 2 life.",
        ),
        plot(
            mana("{2}{B}"),
            "Plot {2}{B} (You may pay {2}{B} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
        ),
    ],
    delayed: &[delayed_trigger(effects::target_player_draws_and_loses_life::<2, 2>, "When you do, target player draws two cards and loses 2 life.")
        .targets(&[targets::PLAYER])],
    ..oracle::UNSCRUPULOUS_CONTRACTOR
};
