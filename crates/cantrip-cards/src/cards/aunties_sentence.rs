use cantrip_api::*;
use cantrip_mechanics::hand::discard_chosen_nonland_permanent_from_revealed_hand;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target opponent reveals their hand. You choose a nonland permanent card from it. That player discards that card."
fn strip(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    discard_chosen_nonland_permanent_from_revealed_hand(g, item, opponent)?;
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(strip, "Target opponent reveals their hand. You choose a nonland permanent card from it. That player discards that card.")
        .targets(&[targets::OPPONENT]),
    mode(effects::target_creature_gets::<-2, -2>, "Target creature gets -2/-2 until end of turn.").targets(&[targets::CREATURE]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::AUNTIES_SENTENCE };
