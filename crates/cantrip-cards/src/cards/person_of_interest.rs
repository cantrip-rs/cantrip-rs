use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, suspect it. Create a 2/2 white and blue Detective creature token." (CR 701.60a, 111.1): two
/// sentences, so two instructions. The token is made even if this creature has already left the battlefield (its ruling).
fn suspect_it_and_create_a_detective(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.suspect(item.object);
    g.end_sentence();
    tokens::create_a_white_blue_detective(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        suspect_it_and_create_a_detective,
        "When this creature enters, suspect it. Create a 2/2 white and blue Detective creature token. (A suspected creature has menace and can't block.)",
    )],
    ..oracle::PERSON_OF_INTEREST
};

pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_BLUE_DETECTIVE];
