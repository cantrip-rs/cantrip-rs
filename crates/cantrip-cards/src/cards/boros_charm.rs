use cantrip_api::*;
use cantrip_mechanics::effects::{self, gain_indestructible_until_end_of_turn};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Permanents you control gain indestructible until end of turn": the permanents there are as it resolves, and only those
/// (CR 611.2c).
fn indestructible(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let permanents = g.permanents_of(item.controller);
    instruction(g, |g| {
        for permanent in permanents {
            gain_indestructible_until_end_of_turn(g, item.object, permanent);
        }
    });
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::damage_first_target::<4>, "Boros Charm deals 4 damage to target player or planeswalker.").targets(&[targets::PLAYER_OR_PLANESWALKER]),
    mode(indestructible, "Permanents you control gain indestructible until end of turn."),
    mode(effects::target_gains_double_strike, "Target creature gains double strike until end of turn.").targets(&[targets::CREATURE]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::BOROS_CHARM };
