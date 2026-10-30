use cantrip_api::*;
use cantrip_mechanics::select::{creature_cards, exactly};
use cantrip_mechanics::{choose, pump, trample_until_end_of_turn};
use cantrip_oracle as oracle;

/// "Choose exactly two creatures you control. You draw X cards and the chosen creatures get +X/+X and gain trample until end
/// of turn, where X is the difference between the chosen creatures' powers." The choice is made as the spell resolves and is
/// not targeting (CR 608.2d); with fewer than two creatures nothing can be chosen and the spell does nothing (the
/// ruling). X is the absolute difference of the two powers as they are when the choice is made, a negative power being what
/// it is (CR 107.1b), and is the same for the draw and for both pumps, which are not affected by each other: it is not read
/// again after the first creature gets +X/+X.
fn choose_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let creatures: Vec<ObjectId> = g.permanents_of(you).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    let chosen = if creatures.len() < 2 { Vec::new() } else { choose::select(g, you, item.object, Purpose::Choose, creatures, creature_cards(exactly(2)))? };
    let &[first, second] = chosen.as_slice() else {
        return Ok(());
    };
    let power = |g: &mut dyn Game, id: ObjectId| i32::from(g.chars(id).power.unwrap_or(0));
    let x = (power(g, first) - power(g, second)).unsigned_abs();
    g.draw(you, x);
    instruction(g, |g| {
        for creature in [first, second] {
            pump(g, item.object, creature, x as i16, x as i16);
            trample_until_end_of_turn(g, item.object, creature);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], choose_two), ..oracle::SPRY_AND_MIGHTY };
