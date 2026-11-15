use cantrip_api::*;
use cantrip_mechanics::{choose, effects, your_turn};
use cantrip_mechanics::library::{Find, search_to_hand};
use cantrip_oracle as oracle;

/// "Search your library for a card, put it into your hand, then shuffle. An opponent gains control of this artifact."
/// The opponent is chosen from those still in the game; one is no choice. Nothing changes hands if the artifact has
/// left the battlefield (CR 608.2b).
fn search_and_give(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    search_to_hand(g, player, item.object, Find::ANY, |_, _| true)?;

    let me = item.object;
    if !is_live(g, me) || !g.is_on_battlefield(me) {
        return Ok(());
    }
    let opponents: Vec<PlayerId> = g.state().opponents(player).collect();
    let new_controller = match opponents.len() {
        1 => Some(opponents[0]),
        _ => choose::player(g, player, me, opponents, "GainControl", 0)?,
    };
    let Some(new_controller) = new_controller else { return Ok(()) };
    g.gain_control(me, me, new_controller, Duration::Indefinite);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "This artifact enters with three wish counters on it" (CR 614.1c), however it enters.
    enters: Some(effects::enters_with_counters::<{ CounterKind::Wish as usize }, 3>),
    abilities: &[ability(cost("{1}, {T}").and(remove_counters(CounterKind::Wish, 1)), search_and_give, "{1}, {T}, Remove a wish counter from this artifact: Search your library for a card, put it into your hand, then shuffle. An opponent gains control of this artifact. Activate only during your turn.")
        .condition(your_turn)],
    ..oracle::WISHCLAW_TALISMAN
};
