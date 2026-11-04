use cantrip_api::*;
use cantrip_mechanics::effects::gain_indestructible_until_end_of_turn;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// A 1/2 white Moogle creature token with lifelink (CR 111.4).
pub static MOOGLE: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(2),
    keywords: Keywords::of(&[Keyword::Lifelink]),
    text: "Lifelink",
    ..token!(Moogle)
};

/// "For each creature you control, create a 1/2 white Moogle creature token with lifelink. Then creatures you control gain
/// indestructible until end of turn." The number of creatures is counted as the spell resolves, before any token is made, and
/// the tokens are made together as one event (CR 111.1). Then, the creatures the controller has - the new tokens among them -
/// gain indestructible (CR 611.2c).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).count();
    tokens::create(g, item.controller, &MOOGLE, count as u32)?;
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

pub static EXTRA: &[&CardDef] = &[&MOOGLE];

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::MOOGLES_VALOR };
