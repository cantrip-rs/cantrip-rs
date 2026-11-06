use cantrip_api::*;
use cantrip_mechanics::{effects, select, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, sacrifice a creature other than this creature. If you can't, this creature deals 7 damage to you." (CR 608.2, 120.3, 701.21a).
fn sacrifice_creature_other_than_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let sacrificed = effects::may_sacrifice(
        g,
        item.controller,
        item.object,
        select::another_creature_permanents(select::as_many_as_can(1)),
    )?;
    g.end_sentence();
    if sacrificed.is_empty() {
        g.damage_together(&[(item.object, Target::Player(item.controller), 7)]);
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            sacrifice_creature_other_than_creature,
            "At the beginning of your upkeep, sacrifice a creature other than this creature. If you can't, this creature deals 7 damage to you.",
        ),
    ],
    ..oracle::LORD_OF_THE_PIT
};
