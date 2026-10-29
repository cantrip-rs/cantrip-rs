use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, targets};
use cantrip_oracle as oracle;

/// Mode 1: "Target player discards any number of cards, then draws that many cards." The target player chooses (CR 701.9); with
/// the target gone the mode does nothing (CR 608.2b).
fn discard_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    effects::discard_any_number_then_draw(g, player, item.object)
}

/// Mode 2: damage equal to the number of instant and sorcery cards in your graveyard, counted as the spell resolves, so mode 1's
/// discards by you count. The number is the damage whether or not the creature is still there (CR 608.2b stops it with no target).
fn damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let n = g
        .state()
        .zone(Zone::graveyard(item.controller))
        .iter()
        .filter(|&&c| cantrip_mechanics::library::instant_or_sorcery_card(g, c))
        .count() as u16;
    damage_target(g, item, 0, n);
    Ok(())
}

/// "Choose one or both" (CR 700.2): the modes are carried out in the order printed (CR 608.2c).
static MODAL: ModalDef = modes(&[
    mode(discard_and_draw, "Target player discards any number of cards, then draws that many cards.").targets(&[targets::PLAYER]),
    mode(damage, "Steal the Show deals damage equal to the number of instant and sorcery cards in your graveyard to target creature or planeswalker.")
        .targets(&[targets::CREATURE_OR_PLANESWALKER]),
])
.choose(1, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::STEAL_THE_SHOW };
