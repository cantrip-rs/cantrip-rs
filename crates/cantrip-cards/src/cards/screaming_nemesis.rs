use cantrip_api::*;
use cantrip_mechanics::{history::players_dealt_damage_by, targets, triggers};
use cantrip_oracle as oracle;

/// It deals that much damage to any other target. If a player is dealt damage this way, they can't gain life for the rest of
/// the game (CR 119.7): each player the damage actually reached, read from the damage events this resolution recorded
/// (CR 120.3) - not a target a redirection (CR 614.1a, Ancient Adamantoise) or a prevention kept it from, and whoever a
/// redirection sent it to.
fn retaliate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let before = g.history().head() as usize;
    g.damage(item.object, item.target(0), item.matched_damage_amount());
    let players = players_dealt_damage_by(g.history(), item.object, before);
    g.end_sentence();
    instruction(g, |g| {
        for &p in &players {
            g.add_effect(item.object, item.object, EffectKind::CantGainLife(p), Duration::Indefinite);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_DEALT_DAMAGE, retaliate, "Whenever this creature is dealt damage, it deals that much damage to any other target. If a player is dealt damage this way, they can't gain life for the rest of the game.").damage_amount()
        .targets(&[targets::ANY_OTHER_TARGET])
        .sums_per_subject()],
    ..oracle::SCREAMING_NEMESIS
};
