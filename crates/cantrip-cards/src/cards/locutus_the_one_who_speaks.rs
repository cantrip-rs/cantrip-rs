use cantrip_api::*;
use cantrip_mechanics::{damage_target, statics, targets};
use cantrip_oracle as oracle;

/// "It deals 2 damage to target creature. You gain 2 life." The card is the source as it last existed in the zone it was
/// discarded from (CR 113.7a, 608.2h); a target that is no longer legal is dealt nothing (CR 608.2b), and the life is
/// gained whether or not the damage was dealt. The two sentences are two instructions (CR 608.2c).
fn deal_two_and_gain_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 2);
    g.end_sentence();
    g.gain_life(item.controller, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{W}{B}").and(DISCARD_THIS),
        deal_two_and_gain_two,
        "{W}{B}, Discard this card: It deals 2 damage to target creature. You gain 2 life.",
    )
    .targets(&[targets::CREATURE])
    .from(ZoneKind::Hand)],
    statics: &[static_ability(
        Layer::PtModify,
        statics::your_artifact_creatures,
        statics::plus_pt::<1, 1>,
        "Artifact creatures you control get +1/+1.",
    )],
    ..oracle::LOCUTUS_THE_ONE_WHO_SPEAKS
};
