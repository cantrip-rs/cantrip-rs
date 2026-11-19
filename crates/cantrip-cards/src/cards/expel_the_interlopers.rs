use cantrip_api::*;
use cantrip_mechanics::{choose, effects};
use cantrip_oracle as oracle;

/// "Choose a number between 0 and 10. Destroy all creatures with power greater than or equal to the chosen number" (CR 107.1,
/// 701.8a, 101.4): the controller chooses on resolution (not a target, not a cost), the endpoints included. Every creature on the
/// battlefield, whoever controls it, whose power is at least the number goes at once, its power as the spell resolves
/// (CR 608.2c): a creature of negative power is below even 0, so it stays.
fn destroy_creatures_with_power_at_least_the_chosen_number(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let chosen = choose::number(g, item.controller, item.object, 0, 10, "Choose a number")?;
    let doomed: Vec<ObjectId> =
        effects::creatures_on_the_battlefield(g).into_iter().filter(|&id| g.chars(id).power.is_some_and(|power| power >= chosen as i32)).collect();
    g.destroy_all(&doomed);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], destroy_creatures_with_power_at_least_the_chosen_number), ..oracle::EXPEL_THE_INTERLOPERS };
