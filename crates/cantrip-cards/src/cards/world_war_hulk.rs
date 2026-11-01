use cantrip_api::*;
use cantrip_mechanics::{double, targets, trample_until_end_of_turn};
use cantrip_oracle as oracle;

/// "The next red or green creature spell you cast this turn": a creature spell of the player that has either color, or
/// both. The color words are those of the Saga's own text (CR 612).
fn red_or_green_creature_spell_you_cast(q: &dyn Query, asker: &Asker, spell: ObjectId, player: PlayerId) -> bool {
    let words = q.words(asker.source);
    let chars = q.chars(spell);
    player == asker.controller && chars.is(CardType::Creature) && (chars.colors.has(words.color(Color::Red)) || chars.colors.has(words.color(Color::Green)))
}

/// I: "The next red or green creature spell you cast this turn can be cast without paying its mana cost." (CR 611.2a, 118.9):
/// an alternative cost the player may use for that spell, which is used up by it whichever way it is cast.
fn chapter_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.next_spell_alternative_this_turn(item, item.controller, 0);
    Ok(())
}

/// II: "Put three +1/+1 counters on target creature you control."
fn chapter_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 3);
    Ok(())
}

/// III: "Choose target creature you control. Until end of turn, double its power and toughness and it gains trample."
/// (CR 701.10a-b, 608.2h): the power and toughness it has as the chapter resolves, read once and fixed.
fn chapter_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    // "Double its power and toughness and it gains trample" is one instruction (CR 608.2c): judged by the state triggers once both are done.
    g.one_instruction(&mut |g| {
        double::power_and_toughness(g, item.object, creature);
        trample_until_end_of_turn(g, item.object, creature);
        Ok(())
    })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, chapter_one, "I \u{2014} The next red or green creature spell you cast this turn can be cast without paying its mana cost."),
        chapter(2, chapter_two, "II \u{2014} Put three +1/+1 counters on target creature you control.").targets(&[targets::CREATURE_YOU_CONTROL]),
        chapter(3, chapter_three, "III \u{2014} Choose target creature you control. Until end of turn, double its power and toughness and it gains trample.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    lasting_spell_alternatives: &[SpellAlternative {
        text: "The next red or green creature spell you cast this turn can be cast without paying its mana cost.",
        applies: red_or_green_creature_spell_you_cast,
        cost: InsteadOfManaCost::WithoutPaying,
        flash: false,
    }],
    ..oracle::WORLD_WAR_HULK
};
