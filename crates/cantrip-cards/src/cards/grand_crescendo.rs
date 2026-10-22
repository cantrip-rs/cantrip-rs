use cantrip_api::*;
use cantrip_mechanics::effects::gain_indestructible_until_end_of_turn;
use cantrip_mechanics::tokens::{self, CITIZEN};
use cantrip_oracle as oracle;

/// "Create X 1/1 green and white Citizen creature tokens. Creatures you control gain indestructible until end of turn."
/// The X tokens are made together, as one event (CR 111.1); the creatures that gain indestructible are the ones the
/// spell's controller has as it resolves, the new tokens among them, and only those (CR 611.2c).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &CITIZEN, item.x as u32)?;
    let mine: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature))
        .collect();
    instruction(g, |g| {
        for creature in mine {
            gain_indestructible_until_end_of_turn(g, item.object, creature);
        }
    });
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&CITIZEN];

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::GRAND_CRESCENDO };
