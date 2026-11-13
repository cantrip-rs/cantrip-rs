use cantrip_api::*;
use cantrip_mechanics::library::look_at_top_may_put_one_back_rest_to_graveyard;
use cantrip_mechanics::raid;
use cantrip_oracle as oracle;

// Deathtouch is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[raid::enters(
        look_at_top_may_put_one_back_rest_to_graveyard::<3>,
        "Raid \u{2014} When this creature enters, if you attacked this turn, look at the top three cards of your library. You may put one of those cards back on top of your library. Put the rest into your graveyard.",
    )],
    ..oracle::GUTLESS_PLUNDERER
};
