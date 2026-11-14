use cantrip_api::*;
use cantrip_mechanics::{effects, select, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, you may sacrifice an artifact or creature. If you do, draw a card." The sacrifice is chosen as
/// the ability resolves and may be declined (CR 608.2d, 701.21a); this creature is itself a creature to sacrifice, and the card
/// is drawn only if a permanent was actually sacrificed (CR 608.2c).
fn sacrifice_to_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = select::artifact_or_creature_permanents(select::up_to(1));
    if !effects::may_sacrifice(g, item.controller, item.object, rule)?.is_empty() {
        effects::draw(g, item, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            sacrifice_to_draw,
            "When this creature enters, you may sacrifice an artifact or creature. If you do, draw a card.",
        ),
        triggered(
            triggers::ANOTHER_CREATURE_YOU_CONTROL_WITH_COUNTERS_DIES,
            effects::put_the_subjects_counters_on_this,
            "Whenever another creature you control dies, if it had counters on it, put its counters on this creature.",
        ),
    ],
    ..oracle::BUZZARD_WASP_COLONY
};
