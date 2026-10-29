use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Put a promotion counter on her. Then if there are three or more promotion counters on her, remove those counters and
/// she deals 2 damage to each opponent." (CR 122.6, 608.2c, 120.2): put first, then check and remove all counters before
/// dealing damage simultaneously to opponents still in the game.
fn promotion_and_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Promotion, 1);
    let promotions = g.state().obj(item.object).counters(CounterKind::Promotion);
    if promotions >= 3 {
        g.remove_counters(item.object, CounterKind::Promotion, promotions);
        effects::damage_each_opponent(g, item, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_BECOMES_TAPPED,
        promotion_and_damage,
        "Whenever Beckett Mariner becomes tapped, put a promotion counter on her. Then if there are three or more promotion counters on her, remove those counters and she deals 2 damage to each opponent.",
    )],
    ..oracle::BECKETT_MARINER_IMPETUOUS_ENSIGN
};
