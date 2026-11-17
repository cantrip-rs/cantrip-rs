use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, another target creature you control gets +1/+1 and gains haste until end of turn" (CR 611.2a,
/// 702.10): the target is chosen as the ability goes on the stack and can't be the Coyote itself.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::target_creature_gets_and_gains_haste::<1, 1>,
            "When this creature enters, another target creature you control gets +1/+1 and gains haste until end of turn.",
        )
        .targets(&[targets::OTHER_CREATURE_YOU_CONTROL]),
        plot(
            mana("{1}{R}"),
            "Plot {1}{R} (You may pay {1}{R} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
        ),
    ],
    ..oracle::CUNNING_COYOTE
};
