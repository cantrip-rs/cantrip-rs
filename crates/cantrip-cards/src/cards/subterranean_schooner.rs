use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Vehicle. "Whenever this Vehicle attacks, target creature that crewed it this turn explores" (CR 701.44a, 702.122c): the target is
/// chosen as the ability goes on the stack, from the creatures tapped to crew it this turn; it explores if it is still a legal target.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS,
            effects::target_creature_explores,
            "Whenever this Vehicle attacks, target creature that crewed it this turn explores. (Reveal the top card of your library. Put that card into your hand if it's a land. Otherwise, put a +1/+1 counter on that creature, then put the card back or put it into your graveyard.)",
        )
        .targets(&[targets::CREATURE_THAT_CREWED_IT]),
        crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::SUBTERRANEAN_SCHOONER
};
