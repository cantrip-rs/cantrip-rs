use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature you control gets +1/+0 and gains deathtouch until end of turn. Untap it." (CR 611.2a, 702.2, 701.26b): the
/// untap is the next sentence (CR 608.2c), the same target if it is still legal as this resolves (CR 608.2b).
fn empower_and_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gets_and_gains_deathtouch::<1, 0>(g, item)?;
    g.end_sentence();
    effects::untap_targets(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOU_ATTACK,
            empower_and_untap,
            "Whenever you attack, target creature you control gets +1/+0 and gains deathtouch until end of turn. Untap it.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
        crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::HAUNTED_HELLRIDE
};
