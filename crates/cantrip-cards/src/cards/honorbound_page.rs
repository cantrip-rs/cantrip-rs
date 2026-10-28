use cantrip_api::*;
use cantrip_mechanics::{effects, prepare, pump, targets};
use cantrip_oracle as oracle;

/// "Forum's Favor - Target creature gets +1/+0 and gains flying until end of turn."
fn forums_favor(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 1, 0);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Flying);
    });
    Ok(())
}

/// Honorbound Page's prepare spell (CR 722): not a card, only ever a copy cast from exile (CR 722.3c).
pub static FORUMS_FAVOR: CardDef = CardDef {
    name: "Forum's Favor",
    cost: mana("{W}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::CREATURE], forums_favor),
    text: "Target creature gets +1/+0 and gains flying until end of turn.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(prepare::enters_prepared),
    prepare: Some(&FORUMS_FAVOR),
    ..oracle::HONORBOUND_PAGE
};

pub static EXTRA: &[&CardDef] = &[&FORUMS_FAVOR];
