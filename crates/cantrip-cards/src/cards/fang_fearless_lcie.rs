use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::CARDS_LEAVE_YOUR_GRAVEYARD;
use cantrip_oracle as oracle;

/// "Whenever one or more cards leave your graveyard, you draw a card and you lose 1 life. This ability triggers only once
/// each turn." Cards that leave together trigger it once (CR 603.2c), and only the first time each turn. Fang is the
/// half of its meld pair that doesn't meld: Vanille, Cheerful l'Cie melds them (CR 701.42a), and Fang alone can't
/// transform (CR 712.4c), like Phyrexian Dragon Engine and The Mightstone and Weakstone.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        CARDS_LEAVE_YOUR_GRAVEYARD,
        effects::draw_a_card_and_lose_1_life,
        "Whenever one or more cards leave your graveyard, you draw a card and you lose 1 life. This ability triggers only once each turn.",
    )
    .once_per_batch()
    .triggers_each_turn(1)],
    ..oracle::FANG_FEARLESS_LCIE
};
