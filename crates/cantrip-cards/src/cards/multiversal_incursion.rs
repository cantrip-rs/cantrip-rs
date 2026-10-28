use cantrip_api::*;
use cantrip_oracle as oracle;

/// "For each nontoken creature you control, create a token that's a copy of that creature, except it isn't legendary."
/// The creatures are found once, as the spell resolves (CR 608.2c), before any token exists; each copy is of its creature's
/// copiable values (CR 707.2) with the exception applied (CR 707.9b), and they enter together as one instruction (CR 101.4,
/// 603.2c).
fn copy_each_nontoken_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let creatures: Vec<ObjectId> = g.permanents_of(you).into_iter().filter(|&c| !g.obj(c).token && g.chars(c).is(CardType::Creature)).collect();
    let copies: Vec<(Copiable, PlayerId)> = creatures.into_iter().map(|c| (g.copiable_values(c).nonlegendary(), you)).collect();
    g.create_token_copies_together(&copies)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], copy_each_nontoken_creature), ..oracle::MULTIVERSAL_INCURSION };
