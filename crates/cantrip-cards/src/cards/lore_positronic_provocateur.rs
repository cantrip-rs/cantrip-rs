use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Target creature gets +1/+0 until end of turn and can't be blocked this turn." (CR 611.2a, 509.1b). Both halves act on the
/// same target: nothing if it is no longer a legal target as the spell resolves (CR 608.2b).
fn pump_and_unblockable(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.one_instruction(&mut |g| {
        effects::target_creature_gets::<1, 0>(g, item)?;
        effects::target_creature_cant_be_blocked_this_turn(g, item)
    })
}

/// "Surveil 2." (CR 701.25a).
fn surveil_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 2)
}

/// "When Lore enters, choose one —" (CR 603.6a, 700.2b). Each mode uses the shared effect and target rules.
static MODAL: ModalDef = modes(&[
    mode(pump_and_unblockable, "Target creature gets +1/+0 until end of turn and can't be blocked this turn.").targets(&[targets::CREATURE]),
    mode(surveil_two, "Surveil 2."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODAL,
        "When Lore enters, choose one — target creature gets +1/+0 until end of turn and can't be blocked this turn; or surveil 2.",
    )],
    ..oracle::LORE_POSITRONIC_PROVOCATEUR
};
