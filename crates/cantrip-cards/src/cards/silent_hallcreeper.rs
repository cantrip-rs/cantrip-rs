use cantrip_api::*;
use cantrip_mechanics::{effects, history, targets};
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_oracle as oracle;

/// "This creature becomes a copy of another target creature you control": the target's copiable values (CR 707.2), with no
/// exception and no duration, so it lasts as long as this permanent stays on the battlefield (CR 611.2a-b); counters and
/// other effects on it stay (the ruling). It doesn't enter, so the copied creature's "enters" abilities don't apply. This
/// ability is not among the copied values, so a copy has none of the choices left. Nothing once the target is gone (CR
/// 608.2b: the core doesn't resolve it).
fn become_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(original) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(original);
    g.become_copy(item.object, item.object, values, Duration::Indefinite);
    Ok(())
}

/// "Choose one that hasn't been chosen" (CR 700.2): a mode already chosen for this permanent's ability is not offered again.
static CHOICE: ModalDef = modes(&[
    mode(effects::two_plus_one_counters_on_this, "Put two +1/+1 counters on this creature."),
    mode(|g, item| effects::draw(g, item, 1), "Draw a card."),
    mode(become_copy, "This creature becomes a copy of another target creature you control.").targets(&[targets::OTHER_CREATURE_YOU_CONTROL]),
])
    .allowed(history::mode_not_chosen_before);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(DEALS_COMBAT_DAMAGE_TO_PLAYER, &CHOICE, "Whenever this creature deals combat damage to a player, choose one that hasn't been chosen \u{2014} put two +1/+1 counters on this creature; or draw a card; or this creature becomes a copy of another target creature you control.")],
    ..oracle::SILENT_HALLCREEPER
};
