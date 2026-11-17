use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, ferocious};
use cantrip_oracle as oracle;

/// "When this creature enters, if you control a creature with power 4 or greater, draw a card." (CR 603.4): an intervening
/// "if", asked as the creature enters and again as the trigger resolves; the creature itself counts when its power is 4 or
/// more. Plot (CR 702.170) is the shared special action.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When this creature enters, if you control a creature with power 4 or greater, draw a card.")
            .condition(ferocious::you_control_asked),
        plot(
            mana("{1}{G}"),
            "Plot {1}{G} (You may pay {1}{G} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
        ),
    ],
    ..oracle::BEASTBOND_OUTCASTER
};
