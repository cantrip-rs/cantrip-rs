use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever Totentanz or another nontoken creature you control dies" (CR 700.4, 603.10a): a nontoken creature under your control as
/// it died, or Totentanz itself whatever it is - a token copy of it included, "nontoken" qualifying only "another ... creature" (CR
/// 201.5) - so a Rat token's death makes nothing. "{1}{B}: Target attacking Rat you control gains
/// deathtouch until end of turn" (CR 702.2, 611.2a): a Rat read through the card's words (CR 612) that is attacking, a Rat that is
/// not attacking or one an opponent controls being no legal target.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_OR_ANOTHER_NONTOKEN_CREATURE_YOU_CONTROL_DIES,
            tokens::create_rats_that_cant_block::<1>,
            "Whenever Totentanz or another nontoken creature you control dies, create a 1/1 black Rat creature token with \"This token can't block.\"",
        ),
        ability(
            cost("{1}{B}"),
            effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Deathtouch]).0 }>,
            "{1}{B}: Target attacking Rat you control gains deathtouch until end of turn.",
        )
        .targets(&[target(targets::attacking_creature_of_subtype_you_control::<{ lands::sub(Subtype::Rat) }>)]),
    ],
    ..oracle::TOTENTANZ_SWARM_PIPER
};

pub static EXTRA: &[&CardDef] = &[&tokens::RAT_CANT_BLOCK];
