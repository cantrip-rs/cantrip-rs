use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT;
use cantrip_oracle as oracle;

/// "You get that many additional upkeep steps after this phase" (CR 500.8, 500.10): one added upkeep step for each point of
/// combat damage Obeka dealt (it rides on the trigger, `event_amount`), each after the combat phase it triggered in. An upkeep
/// step only exists in a beginning phase, so each is a further beginning phase holding just that step (the rulings: no untap or
/// draw step). The most recently added happens first among phases at the same point (the rulings).
fn additional_upkeep_steps(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let steps = vec![PhaseSpec::Only(Step::Upkeep); item.matched_damage_amount() as usize];
    g.add_phases(PhaseAnchor::Current, &steps);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT,
        additional_upkeep_steps,
        "Whenever Obeka deals combat damage to a player, you get that many additional upkeep steps after this phase.",
    )
    .damage_amount()],
    ..oracle::OBEKA_SPLITTER_OF_SECONDS
};
