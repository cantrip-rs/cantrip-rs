use cantrip_api::*;
use cantrip_mechanics::an_opponent_controls_more_lands;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "When this creature enters, if an opponent controls more lands than you, create a Treasure token": an intervening
    // "if" (CR 603.4), checked when it would trigger and again as it resolves. Defender is printed.
    abilities: &[triggered(THIS_ENTERS, tokens::create_a_treasure, "When this creature enters, if an opponent controls more lands than you, you create a Treasure token.")
        .condition(an_opponent_controls_more_lands)],
    ..oracle::TICKET_TORTOISE
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
