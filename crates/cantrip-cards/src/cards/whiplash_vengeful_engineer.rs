use cantrip_api::*;
use cantrip_mechanics::{attach, mana, triggers};
use cantrip_oracle as oracle;

/// "If he's equipped" (CR 603.4, 301.5a): an intervening "if", checked as the creature attacks and again as the ability
/// resolves. An Equipment attached to him, whoever controls it.
fn is_equipped(q: &dyn Query, asker: &Asker) -> bool {
    attach::is_equipped(q, asker.source)
}

/// "Each opponent loses X life and you gain X life, where X is the number of Equipment attached to him." (CR 119.3, 608.2h): X is
/// counted as the ability resolves; the life lost and gained are the same X. Whiplash is the one the Equipment is attached to,
/// and the ability does not resolve if he is no longer equipped (CR 603.4).
fn drain_for_each_equipment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = attach::equipment_attached_to(g, item.object).len() as i32;
    g.each_opponent_loses_life(item.controller, x);
    g.gain_life(item.controller, x);
    Ok(())
}

/// "Whiplash enters tapped" (CR 614.1d).
pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        drain_for_each_equipment,
        "Whenever Whiplash attacks, if he's equipped, each opponent loses X life and you gain X life, where X is the number of Equipment attached to him.",
    )
    .condition(is_equipped)],
    ..oracle::WHIPLASH_VENGEFUL_ENGINEER
};
