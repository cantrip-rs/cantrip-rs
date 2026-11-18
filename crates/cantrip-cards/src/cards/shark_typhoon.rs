use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// A blue Shark creature token with flying: the generic definition, its size X/X given as it is made (`shark`).
pub static SHARK: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..token!(Shark)
};

/// "Create an X/X blue Shark creature token with flying" for `controller`, X being `size`. The token is X/X as it is made
/// (CR 111.4, 707.2), never a 0/0 for a moment; an X of 0 makes a 0/0 that state-based actions then put into the graveyard
/// (CR 704.5f). A size past what power and toughness hold (RX-5) is made at that limit, never wrapped. The token is what
/// this card's text says, a text change of it included (CR 612.4).
fn shark(g: &mut dyn Game, item: &StackItem, size: u32) -> Result<(), Suspend> {
    let size = size as i32;
    tokens::create_sized(g, item.object, item.controller, &SHARK, (size, size), 1)?;
    Ok(())
}

/// "... where X is that spell's mana value": the spell's as the ability resolves, or as it last was on the stack if it has
/// left (CR 608.2h: the ability resolves even if that spell is countered) - an {X} in its cost counting as the X announced (CR 202.3e).
fn shark_of_spell(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(spell) = item.subject else { return Ok(()) };
    let size = g.mana_value(spell);
    shark(g, item, size)
}

/// "When you cycle this card, create an X/X ...": X is the X of the cycling cost paid (CR 107.3a), `data[0]`
/// (`triggers::YOU_CYCLE_THIS_CARD`).
fn shark_of_cycling(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    shark(g, item, item.data[0])
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::NONCREATURE_SPELL_YOU_CAST_ABOUT_IT,
            shark_of_spell,
            "Whenever you cast a noncreature spell, create an X/X blue Shark creature token with flying, where X is that spell's mana value.",
        ),
        triggers::cycling(mana("{X}{1}{U}"), "Cycling {X}{1}{U} ({X}{1}{U}, Discard this card: Draw a card.)"),
        triggered(
            triggers::YOU_CYCLE_THIS_CARD,
            shark_of_cycling,
            "When you cycle this card, create an X/X blue Shark creature token with flying.",
        ),
    ],
    ..oracle::SHARK_TYPHOON
};

pub static EXTRA: &[&CardDef] = &[&SHARK];
