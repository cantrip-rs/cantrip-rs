use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, ferocious, targets};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): Stolen Goodies, "Distribute three +1/+1 counters among any number of target creatures you control"
/// (CR 601.2d): up to three targets, as each needs at least one counter.
pub static STOLEN_GOODIES: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL.between(0, 3)], effects::distribute_plus_one_counters),
    divide: Some(Divide { total: 3, reads: None }),
    ..oracle::STOLEN_GOODIES
};

/// "...while you control a creature with power 4 or greater": asked as the attack is declared, as part of the trigger event, and
/// never again as the ability resolves (this is no intervening "if", CR 603.4). This creature may be the big one itself.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        |g, item| effects::this_gains_until_end_of_turn(g, item, Keyword::DoubleStrike),
        "Whenever this creature attacks while you control a creature with power 4 or greater, this creature gains double strike until end of turn.",
    )
    .only_if(ferocious::you_control_asked)],
    adventure: Some(&STOLEN_GOODIES),
    ..oracle::PICNIC_RUINER
};

pub static EXTRA: &[&CardDef] = &[&STOLEN_GOODIES];
