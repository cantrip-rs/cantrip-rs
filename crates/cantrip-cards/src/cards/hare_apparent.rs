use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{library, tokens};
use cantrip_oracle as oracle;

const NAME: &str = "Hare Apparent";

/// "Create a number of 1/1 white Rabbit creature tokens equal to the number of other creatures you control named Hare Apparent"
/// (CR 111.1, 201.2a): counted as the ability resolves, off the board as it stands, so a Hare that left in response is not counted.
/// "Other" is any creature but this object, a name is the one a creature has now (CR 612.8), and only the controller's creatures
/// count; with none, a batch of no tokens is created, which is nothing.
fn make_a_rabbit_for_each_other_hare(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let others = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| id != item.object && g.chars(id).is(CardType::Creature) && library::object_names(g, id).contains(&NAME))
        .count() as u32;
    tokens::create(g, item.controller, &tokens::RABBIT, others)?;
    Ok(())
}

/// "A deck can have any number of cards named Hare Apparent" is a deck-construction rule (CR 100.2a), not an ability: it is in
/// no game and no line of code here.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        make_a_rabbit_for_each_other_hare,
        "When this creature enters, create a number of 1/1 white Rabbit creature tokens equal to the number of other creatures you control named Hare Apparent.",
    )],
    ..oracle::HARE_APPARENT
};

pub static EXTRA: &[&CardDef] = &[&tokens::RABBIT];
