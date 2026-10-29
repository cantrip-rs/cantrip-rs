use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Return each chosen creature to your hand. At the beginning of the next upkeep, create a 4/4 white Bird Soldier
/// creature token with flying for each creature returned to your hand this way." The targets still legal go back together
/// (CR 608.2b); those that reached their owner's hand, the caster's, are remembered for the delayed trigger (CR 603.7),
/// a token among them too, though it then ceases to exist (CR 111.8, 704.5d). Kicked with no target, nothing is returned
/// and the delayed trigger still comes, for no token.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let chosen = targets::legal_objects(g, item);
    let returned: Vec<ObjectId> = g.return_to_hand(&chosen).into_iter().filter(|&new| g.obj(new).zone == Zone::hand(you)).collect();
    let this = g.obj(item.object).card;
    g.delayed_trigger_about(DelayedWhen::Step { step: Step::Upkeep, on_turn_of: None }, item.object, you, AbilityRef::delayed(this, 0), &returned);
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
    spell: spell(&[targets::CREATURE_YOU_OWN.if_kicked(0, TargetSpec::ANY_NUMBER)], effect),
    kicker: kicker(mana("{2}{W}{W}")),
    delayed: &[delayed_trigger(
        create_birds,
        "At the beginning of the next upkeep, create a 4/4 white Bird Soldier creature token with flying for each creature returned to your hand this way.",
    )],
    ..oracle::THE_EAGLES_ARE_COMING
};

pub static EXTRA: &[&CardDef] = &[&BIRD_SOLDIER];
