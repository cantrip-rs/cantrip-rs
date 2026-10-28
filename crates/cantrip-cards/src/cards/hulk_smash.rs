use cantrip_api::*;
use cantrip_mechanics::{destroy_target, fight::bite, targets, teamwork};
use cantrip_oracle as oracle;

/// "Destroy target noncreature artifact" (CR 701.8, 608.2b): nothing if the target is no longer a legal one. With teamwork
/// this mode is chosen beside the other and has the target of its own slot (CR 700.2c).
fn destroy_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

/// "Target creature you control deals damage equal to its power to target creature an opponent controls": one-sided, the creature
/// being the source of the damage (CR 120.3, 609.7), its power read as the spell resolves (`fight::bite`). If either target is
/// illegal by then no damage is dealt (CR 608.2b).
fn deal_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let (Some(creature), Some(victim)) = (targets::object(g, item, 0), targets::object(g, item, 1)) {
        bite(g, creature, victim);
    }
    Ok(())
}

/// "Choose one. If this spell was cast using teamwork, choose both instead." (CR 700.2, 702.194b): with teamwork both modes, each
/// with its own targets (CR 700.2c), done in printed order (CR 700.2d).
static MODAL: ModalDef = modes(&[
    mode(destroy_artifact, "Destroy target noncreature artifact.").targets(&[targets::NONCREATURE_ARTIFACT]),
    mode(deal_damage, "Target creature you control deals damage equal to its power to target creature an opponent controls.").targets(&[
        targets::CREATURE_YOU_CONTROL.labelled("Creature you control to deal damage equal to its power"),
        targets::CREATURE_AN_OPPONENT_CONTROLS.labelled("Creature an opponent controls to be dealt that damage"),
    ]),
])
.when_paid(2, 2);

/// Teamwork 4 (as an additional cost to cast this spell, you may tap any number of creatures you control with total power 4 or
/// more): announced before the modes, since how many are chosen is defined by it (CR 601.2b).
pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    optional_cost: Some(teamwork::optional(4)),
    labels: &["Cast without teamwork (choose one)", "Teamwork 4: Tap any number of creatures you control with total power 4 or more (choose both)"],
    ..oracle::HULK_SMASH
};
