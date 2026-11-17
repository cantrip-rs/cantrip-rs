use cantrip_api::*;
use cantrip_mechanics::{choose, tokens};
use cantrip_oracle as oracle;

/// "Reveal the top five cards of your library and separate them into two piles. An opponent chooses one of those piles. Put that pile into
/// your hand and the other into your graveyard. Create a 0/0 colorless Thopter artifact creature token with flying, then put a +1/+1 counter
/// on it for each card put into your graveyard this way." (CR 701.20a, 700.3, 608.2c-d, 111.1, 122.1): you separate the cards, either pile
/// possibly empty (the rulings), and the opponent you choose as it resolves chooses a pile (`choose::reveal_top_cards_two_piles_one_to_hand_other_to_graveyard`,
/// which says how many cards went to the graveyard); the Thopter is made whatever happened, a 0/0 that dies if none did (CR 704.5f).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let buried = choose::reveal_top_cards_two_piles_one_to_hand_other_to_graveyard::<5, true>(g, item)?;
    tokens::create_with_counters(g, item.controller, &THOPTER_0_0, buried as i32)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::INTRUDE_ON_THE_MIND };

/// The 0/0 colorless Thopter artifact creature token with flying (CR 111).
pub static THOPTER_0_0: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..token!(Thopter)
};

pub static EXTRA: &[&CardDef] = &[&THOPTER_0_0];
