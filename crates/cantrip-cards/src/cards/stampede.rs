use cantrip_api::*;
use cantrip_mechanics::{pump, trample_until_end_of_turn};
use cantrip_oracle as oracle;

/// "Attacking creatures get +1/+0 and gain trample until end of turn." (CR 611.2a, 611.2c).
fn attacking_creatures_get_1_0(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let attackers: Vec<ObjectId> = g.state().attackers.iter().map(|a| a.id)
        .filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for id in attackers {
            pump(g, item.object, id, 1, 0);
            trample_until_end_of_turn(g, item.object, id);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], attacking_creatures_get_1_0),
    ..oracle::STAMPEDE
};
