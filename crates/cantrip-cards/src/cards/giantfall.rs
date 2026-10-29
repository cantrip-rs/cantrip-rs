use cantrip_api::*;
use cantrip_mechanics::{destroy_target, fight, targets};
use cantrip_oracle as oracle;

/// "Target creature you control deals damage equal to its power to target creature an opponent controls" (CR 120.3, 609.7, 608.2b):
/// one-sided, the creature being the source of the damage (`fight::targets_bite`), its power the one it has as the spell resolves.
/// If either target is illegal by then no damage is dealt.
fn deal_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    fight::targets_bite(g, item, 0, 1);
    Ok(())
}

/// "Destroy target artifact" (CR 701.8, 608.2b): nothing if the target is no longer legal.
fn destroy_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(deal_damage, "Target creature you control deals damage equal to its power to target creature an opponent controls.").targets(&[
        targets::CREATURE_YOU_CONTROL.labelled("Creature you control to deal damage equal to its power"),
        targets::CREATURE_AN_OPPONENT_CONTROLS.labelled("Creature an opponent controls to be dealt that damage"),
    ]),
    mode(destroy_artifact, "Destroy target artifact.").targets(&[targets::ARTIFACT]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::GIANTFALL };
