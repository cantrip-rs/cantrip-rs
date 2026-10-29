use cantrip_api::*;
use cantrip_mechanics::effects::gain_until_end_of_turn;
use cantrip_mechanics::triggers::{THIS_ATTACKS, THIS_ENTERS};
use cantrip_mechanics::{pump, tokens};
use cantrip_oracle as oracle;

/// The 2/2 white Dinosaur Soldier creature token it makes.
pub static DINOSAUR_SOLDIER: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Dinosaur Soldier)
};

/// "Create X 2/2 white Dinosaur Soldier creature tokens": X is the X it was cast with, and 0 when it was put onto the
/// battlefield some other way (CR 107.3m); a triggered ability's `counted` is its source's X, read as it triggered.
fn create_x_dinosaur_soldiers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &DINOSAUR_SOLDIER, item.counted as u32)?;
    Ok(())
}

/// "Dinosaurs you control other than this creature get +1/+1 and gain flying until end of turn" (CR 611.2c): the Dinosaurs
/// its controller controls as the ability resolves, and none that enters later.
fn other_dinosaurs_get_plus_one_and_fly(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let dinosaur = g.words(item.object).subtype(Subtype::Dinosaur);
    let dinosaurs: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| id != item.object && g.chars(id).is(CardType::Creature) && g.chars(id).has_subtype(dinosaur))
        .collect();
    instruction(g, |g| {
        for id in dinosaurs {
            pump(g, item.object, id, 1, 1);
            gain_until_end_of_turn(g, item.object, id, Keyword::Flying);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, create_x_dinosaur_soldiers, "When this creature enters, create X 2/2 white Dinosaur Soldier creature tokens."),
        triggered(THIS_ATTACKS, other_dinosaurs_get_plus_one_and_fly, "Whenever this creature attacks, Dinosaurs you control other than this creature get +1/+1 and gain flying until end of turn."),
    ],
    ..oracle::TRICERATON_COMMANDER
};

pub static EXTRA: &[&CardDef] = &[&DINOSAUR_SOLDIER];
