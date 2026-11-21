use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_mechanics::triggers::THIS_BECOMES_PLOTTED;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "When this card becomes plotted, it deals 2 damage to any target." (CR 702.170a, 603.1, 120.3): the card's own ability, functioning
/// in exile where the plot action has just put it (CR 113.6k), so the card is the source of the damage. It triggers however the card
/// becomes plotted. Reach is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_BECOMES_PLOTTED, effects::damage_first_target::<2>, "When this card becomes plotted, it deals 2 damage to any target.")
            .targets(&[targets::ANY]),
        plot(mana("{3}{R}"), "Plot {3}{R} (You may pay {3}{R} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)"),
    ],
    ..oracle::LONGHORN_SHARPSHOOTER
};
