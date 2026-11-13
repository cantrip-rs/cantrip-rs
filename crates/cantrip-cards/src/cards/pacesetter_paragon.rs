use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{R}"),
        effects::plus_one_counters_on_this_that_gains_until_end_of_turn::<1, { Keywords::of(&[Keyword::DoubleStrike]).0 }>,
        "Exhaust \u{2014} {2}{R}: Put a +1/+1 counter on this creature. It gains double strike until end of turn. (Activate each exhaust ability only once.)",
    )
    .exhaust()],
    ..oracle::PACESETTER_PARAGON
};
