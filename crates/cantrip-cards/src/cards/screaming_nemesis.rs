use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// It deals that much damage to any other target. If a player is dealt damage this way, they can't gain life for the rest of
/// the game (CR 119.7): each player the damage actually reached, read from the damage events this resolution recorded
/// (CR 120.3) - not a target a redirection (CR 614.1a, Ancient Adamantoise) or a prevention kept it from, and whoever a
/// redirection sent it to.
fn retaliate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let before = g.history().head() as usize;
    g.damage(item.object, item.target(0), item.data[0] as u16);
    let dealt_to = |r: &Record| match r.event {
        Event::Damage { source, target: Target::Player(p), amount, .. } if source == item.object && amount > 0 => Some(p),
        _ => None,
    };
    let players: Vec<PlayerId> = g.history().all()[before..].iter().filter_map(dealt_to).collect();
    g.end_sentence();
    instruction(g, |g| {
        for (i, &p) in players.iter().enumerate() {
            if !players[..i].contains(&p) {
                g.add_effect(item.object, item.object, EffectKind::CantGainLife(p), Duration::Indefinite);
            }
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_DEALT_DAMAGE, retaliate, "Whenever this creature is dealt damage, it deals that much damage to any other target. If a player is dealt damage this way, they can't gain life for the rest of the game.")
        .targets(&[targets::ANY_OTHER_TARGET])
        .sums_per_subject()],
    ..oracle::SCREAMING_NEMESIS
};
