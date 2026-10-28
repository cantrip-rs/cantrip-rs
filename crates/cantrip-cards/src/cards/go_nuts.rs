use cantrip_api::*;
use cantrip_mechanics::{effects, fight, targets, teamwork};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on target creature." (CR 122.1, 608.2b)
fn counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_target(g, item)
}

/// "Target creature you control fights target creature an opponent controls." (CR 701.14a-b): neither deals damage when either
/// target is no longer legal.
fn fight_targets(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    fight::targets_fight(g, item, 0, 1);
    Ok(())
}

/// "Choose one. If this spell was cast using teamwork, choose both instead." (CR 700.2, 702.194b): with teamwork both modes, each
/// with its own targets (CR 700.2c), done in printed order (CR 700.2d), so the counter lands before the fight.
static MODAL: ModalDef = modes(&[
    mode(counter, "Put a +1/+1 counter on target creature.").targets(&[targets::CREATURE]),
    mode(fight_targets, "Target creature you control fights target creature an opponent controls.")
        .targets(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS]),
])
.when_paid(2, 2);

/// Teamwork 3 (as an additional cost to cast this spell, you may tap any number of creatures you control with total power 3 or
/// more): announced before the modes, since how many are chosen is defined by it (CR 601.2b).
pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    optional_cost: Some(teamwork::optional(3)),
    labels: &["Cast without teamwork (choose one)", "Teamwork 3: Tap any number of creatures you control with total power 3 or more (choose both)"],
    ..oracle::GO_NUTS
};
