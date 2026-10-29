use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// Indestructible is the core's (CR 702.12b). "{2}, {T}, Sacrifice this artifact: You gain 3 life." is an activated ability like a
/// Food token's (CR 111.10b); "When this artifact is put into a graveyard from the battlefield, draw a card" is looked back on
/// (CR 603.10a), so sacrificing it as that cost still draws, and any other way to the graveyard does too - not exile, not the hand.
pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{2}, {T}").and(SACRIFICE_THIS), effects::gain_three_life, "{2}, {T}, Sacrifice this artifact: You gain 3 life."),
        triggered(THIS_DIES, |g, item| effects::draw(g, item, 1), "When this artifact is put into a graveyard from the battlefield, draw a card."),
    ],
    ..oracle::NUTRIENT_BLOCK
};
