use cantrip_api::*;
use cantrip_mechanics::{effects, prepare, targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature you control gets +2/+0 and gains deathtouch until end of turn" (CR 611.2a), if it is still a legal
/// target as this resolves (CR 608.2b).
fn venomous_words(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        cantrip_mechanics::pump(g, item.object, creature, 2, 0);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Deathtouch);
    });
    Ok(())
}

/// Scathing Shadelock's prepare spell (CR 722): not a card here, only ever a copy.
pub static VENOMOUS_WORDS: CardDef = CardDef {
    name: "Venomous Words",
    cost: mana("{B}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::CREATURE_YOU_CONTROL], venomous_words),
    text: "Target creature you control gets +2/+0 and gains deathtouch until end of turn.",
    ..CardDef::BLANK
};

/// "It becomes prepared" (CR 722.3a): nothing if it left the battlefield, or already is prepared.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_FIRST_MAIN_PHASE,
        prepare::become_prepared,
        "At the beginning of your first main phase, this creature becomes prepared. (While it's prepared, you may cast a copy of its spell. Doing so unprepares it.)",
    )],
    prepare: Some(&VENOMOUS_WORDS),
    ..oracle::SCATHING_SHADELOCK
};

pub static EXTRA: &[&CardDef] = &[&VENOMOUS_WORDS];
