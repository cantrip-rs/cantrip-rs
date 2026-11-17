use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_mechanics::triggers::THIS_BECOMES_PLOTTED;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "When this card becomes plotted, target creature gets +3/+2 and gains trample until end of turn" (CR 702.170a, 603.1, 611.2a): the
/// card's own ability, functioning in exile where the plot action has just put it (CR 113.6k), so it triggers from there and its
/// target is any creature. It triggers however the card becomes plotted: its own plot action, or an effect that plots it, such as
/// Aven Interrupter's (CR 702.170c). The creature spell cast later from exile for free is no plotting, and a card put into exile any
/// other way does not trigger it. Trample is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_BECOMES_PLOTTED,
            effects::target_creature_gets_and_gains_trample::<3, 2>,
            "When this card becomes plotted, target creature gets +3/+2 and gains trample until end of turn.",
        )
        .targets(&[targets::CREATURE]),
        plot(mana("{1}{G}"), "Plot {1}{G} (You may pay {1}{G} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)"),
    ],
    ..oracle::ALOE_ALCHEMIST
};
