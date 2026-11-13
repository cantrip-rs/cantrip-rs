use cantrip_api::*;
use cantrip_mechanics::{exchange, targets};
use cantrip_oracle as oracle;

/// "Exchange control of two target creatures." (CR 701.12a-b): two different creatures (CR 601.2c), either anyone's (the
/// ruling), both still legal targets as it resolves or none of it happens (the card's ruling).
pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE.between(2, 2)], exchange::control_of_two_targets), ..oracle::SWITCHEROO };
