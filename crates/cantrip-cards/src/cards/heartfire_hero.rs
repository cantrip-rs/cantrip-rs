use cantrip_api::*;
use cantrip_mechanics::effects::{damage_each_opponent, plus_one_counter_on_this};
use cantrip_mechanics::targets::power_or_last_known;
use cantrip_mechanics::triggers::{THIS_DIES, VALIANT};
use cantrip_oracle as oracle;

/// "When this creature dies, it deals damage equal to its power to each opponent." Its power is as it last existed on the
/// battlefield (CR 608.2h, 113.7a), counters included; it is the source of the damage, dealt to every opponent together
/// (CR 120.2).
fn blast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = power_or_last_known(g, item.object).max(0) as u32;
    damage_each_opponent(g, item, power);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            VALIANT,
            plus_one_counter_on_this,
            "Valiant \u{2014} Whenever this creature becomes the target of a spell or ability you control for the first time each turn, put a +1/+1 counter on it.",
        ),
        triggered(THIS_DIES, blast, "When this creature dies, it deals damage equal to its power to each opponent."),
    ],
    ..oracle::HEARTFIRE_HERO
};
