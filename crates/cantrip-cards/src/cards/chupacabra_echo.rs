use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::target_creature_gets_minus_x_x_for_permanent_cards_in_your_graveyard,
        "Fathomless descent \u{2014} When this creature enters, target creature an opponent controls gets -X/-X until end of turn, where X is the number of permanent cards in your graveyard.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::CHUPACABRA_ECHO
};
