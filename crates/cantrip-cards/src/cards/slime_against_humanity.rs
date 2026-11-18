use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// The 0/0 green Ooze creature token with trample (CR 111.1, 702.19a) Slime Against Humanity makes.
static OOZE: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    keywords: Keywords::of(&[Keyword::Trample]),
    ..token!(Ooze)
};

/// "Cards you own in exile and in your graveyard that are Oozes or are named Slime Against Humanity" (CR 108.3, 201.2): a card
/// is owned by its owner wherever it is, so it is the controller's own graveyard and every exile zone, counting the cards the
/// controller owns there. The spell itself is still on the stack, so it is not counted. A token is no card and is never there. "Ooze" is
/// a word of the spell's text, so it is read through the spell's words (CR 612.2), not as the literal subtype.
fn oozes_and_slimes_owned(q: &dyn Query, you: PlayerId, ooze: Subtype) -> i32 {
    let in_exile = q.state().seats().flat_map(|player| q.state().zone(Zone::exile(player)).to_vec());
    let in_graveyard = q.state().zone(Zone::graveyard(you)).to_vec();
    in_exile
        .chain(in_graveyard)
        .filter(|&card| q.state().obj(card).owner == you)
        .filter(|&card| q.chars(card).has_subtype(ooze) || q.chars(card).name == oracle::SLIME_AGAINST_HUMANITY.name)
        .count() as i32
}

/// "Create a 0/0 green Ooze creature token with trample. Put X +1/+1 counters on it, where X is two plus the total number of
/// cards you own in exile and in your graveyard that are Oozes or are named Slime Against Humanity." Two sentences: the token is
/// made first, then the counters go on it (CR 608.2c). Making a token moves no card, so X counted before it is made is the X counted
/// as the second instruction is carried out.
fn create_an_ooze_with_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let ooze = g.words(item.object).subtype(Subtype::Ooze);
    let x = 2 + oozes_and_slimes_owned(g, item.controller, ooze);
    tokens::create_with_counters(g, item.controller, &OOZE, x)?;
    Ok(())
}

/// "A deck can have any number of cards named Slime Against Humanity" is a deck-construction rule (CR 100.2a), not a game rule.
pub static CARD: CardDef = CardDef { spell: spell(&[], create_an_ooze_with_counters), ..oracle::SLIME_AGAINST_HUMANITY };

pub static EXTRA: &[&CardDef] = &[&OOZE];
