use cantrip_api::*;
use cantrip_mechanics::{effects, firebending};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        firebending::trigger::<2>("Firebending 2 (Whenever this creature attacks, add {R}{R}. This mana lasts until end of combat.)"),
        ability(
            cost("{8}"),
            effects::plus_one_counters_on_this_that_gains_until_end_of_turn::<2, { Keywords::of(&[Keyword::Trample]).0 }>,
            "Exhaust \u{2014} {8}: Put two +1/+1 counters on this creature. It gains trample until end of turn. (Activate each exhaust ability only once.)",
        )
        .exhaust(),
    ],
    ..oracle::ROUGH_RHINO_CAVALRY
};
