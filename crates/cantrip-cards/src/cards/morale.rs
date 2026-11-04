use cantrip_api::*;
use cantrip_mechanics::pump;
use cantrip_oracle as oracle;

/// "Attacking creatures get +1/+1 until end of turn." (CR 608.2, 611.2a).
fn attacking_creatures_get_1_1(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let attackers: Vec<ObjectId> = g.state().attackers.iter().map(|a| a.id)
        .filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(
        g,
        |g| {
            for id in attackers {
                pump(g, item.object, id, 1, 1);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], attacking_creatures_get_1_1),
    ..oracle::MORALE
};
