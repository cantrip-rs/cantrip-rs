use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Return each chosen creature to your hand. At the beginning of the next upkeep, create a 4/4 white Bird Soldier
/// creature token with flying for each creature returned to your hand this way." The targets still legal go back together
/// (CR 608.2b); those that reached their owner's hand, the caster's, are remembered for the delayed trigger (CR 603.7),
/// a token among them too, though it then ceases to exist (CR 111.8, 704.5d). A merged creature is one creature returned
/// (CR 730.2), remembered by its own move's card, however many cards it became (CR 730.3c). Kicked with no target, nothing
/// is returned and the delayed trigger still comes, for no token.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let chosen = targets::legal_objects(g, item);
    let moved = g.return_to_hand(&chosen);
    let returned: Vec<ObjectId> = g
        .permanents_left(&moved)
        .into_iter()
        .filter_map(|old| g.obj(old).became)
        .filter(|&new| g.obj(new).zone == Zone::hand(you))
        .collect();
    cantrip_mechanics::delayed::at_next_upkeep_about(g, item, 0, &returned);
    Ok(())
}

/// "Create a 4/4 white Bird Soldier creature token with flying for each creature returned to your hand this way."
fn create_birds(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = g.event_objects(item).len() as u32;
    tokens::create(g, item.controller, &BIRD_SOLDIER, count)?;
    Ok(())
}

/// The 4/4 white Bird Soldier creature token with flying it makes.
static BIRD_SOLDIER: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..token!(Bird Soldier)
};

pub static CARD: CardDef = CardDef {
    // One target creature you own; kicked, any number of them (CR 702.33a, 601.2c).
    spell: spell(&[targets::CREATURE_YOU_OWN.if_paid(0, TargetSpec::ANY_NUMBER)], effect),
    kicker: kicker(mana("{2}{W}{W}")),
    delayed: &[delayed_trigger(
        create_birds,
        "At the beginning of the next upkeep, create a 4/4 white Bird Soldier creature token with flying for each creature returned to your hand this way.",
    )],
    ..oracle::THE_EAGLES_ARE_COMING
};

pub static EXTRA: &[&CardDef] = &[&BIRD_SOLDIER];
