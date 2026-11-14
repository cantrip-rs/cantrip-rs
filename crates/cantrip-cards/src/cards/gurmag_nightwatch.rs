use cantrip_api::*;
use cantrip_mechanics::library::look_at_top_may_put_one_back_rest_to_graveyard;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        look_at_top_may_put_one_back_rest_to_graveyard::<3>,
        "When this creature enters, look at the top three cards of your library. You may put one of those cards back on top of your library. Put the rest into your graveyard.",
    )],
    ..oracle::GURMAG_NIGHTWATCH
};
