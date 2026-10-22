use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Create X 1/1 white Soldier creature tokens. If X is 5 or more, destroy all other creatures." The Soldiers are made
/// first, together, and then every creature that is not one of them is destroyed together, with nothing cast or
/// activated between the two (ruling 2009-02-01, CR 701.8a). "If X is 5 or more" reads the number chosen for X, not the
/// mana actually spent (ruling 2009-02-01).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let made = tokens::create(g, item.controller, &tokens::SOLDIER, item.x as u32)?;
    if item.x < 5 {
        return Ok(());
    }
    let others: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature) && !made.contains(&id))
        .collect();
    g.destroy_all(&others);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::MARTIAL_COUP };
