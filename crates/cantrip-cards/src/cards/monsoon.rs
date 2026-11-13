use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// Tap the active player's untapped Islands together, then deal damage for those actually tapped (CR 701.26a, 120.3).
fn tap_islands_and_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(player) = item.subject else { return Ok(()) };
    if !g.state().in_game(player) {
        return Ok(());
    }
    let islands: Vec<_> = g.state().battlefield().iter().copied()
        .filter(|&id| !g.obj(id).tapped && g.controller(id) == player
            && g.chars(id).has_subtype(g.words(item.object).subtype(Subtype::Island)))
        .collect();
    let before = g.history().head() as usize;
    g.tap_together(&islands);
    let tapped = g.history().all()[before..].iter()
        .filter(|r| matches!(r.event, Event::Tapped { permanent } if islands.contains(&permanent)))
        .count() as u32;
    g.end_sentence();
    g.damage(item.object, Target::Player(player), tapped);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EACH_PLAYERS_END_STEP,
        tap_islands_and_damage,
        "At the beginning of each player's end step, tap all untapped Islands that player controls and this enchantment deals X damage to the player, where X is the number of Islands tapped this way.",
    )],
    ..oracle::MONSOON
};
