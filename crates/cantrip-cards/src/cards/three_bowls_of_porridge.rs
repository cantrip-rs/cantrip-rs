use cantrip_api::*;
use cantrip_mechanics::{effects, history, targets};
use cantrip_oracle as oracle;

/// "Sacrifice this artifact. You gain 3 life." (CR 701.21a, 119.3): the life is gained whether or not it could be sacrificed (it
/// may have left the battlefield, or another player may control it now), a sentence of its own.
fn sacrifice_and_gain_3_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::sacrifice_self(g, item)?;
    g.end_sentence();
    effects::gain_life(g, item, 3)
}

/// "Choose one that hasn't been chosen" (CR 700.2): the modes this artifact has chosen before, whoever chose them and whatever
/// became of those abilities, are read from the event history (`history::mode_not_chosen_before`). A mode with targets is offered
/// only while it has a legal one (CR 601.2c). With every mode chosen the ability cannot be activated.
static MODAL: ModalDef = modes(&[
    mode(effects::damage_first_target::<2>, "This artifact deals 2 damage to target creature.").targets(&[targets::CREATURE]),
    mode(effects::tap_targets, "Tap target creature.").targets(&[targets::CREATURE]),
    mode(sacrifice_and_gain_3_life, "Sacrifice this artifact. You gain 3 life."),
])
.allowed(history::mode_not_chosen_before);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_ability(
        cost("{2}").and(TAP),
        &MODAL,
        "{2}, {T}: Choose one that hasn't been chosen \u{2014} This artifact deals 2 damage to target creature; or tap target creature; or sacrifice this artifact and you gain 3 life.",
    )],
    ..oracle::THREE_BOWLS_OF_PORRIDGE
};
