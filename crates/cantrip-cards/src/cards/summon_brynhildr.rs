use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, library, triggers};
use cantrip_oracle as oracle;

/// I: "Exile the top card of your library. During any turn you put a lore counter on this Saga, you may play that card"
/// (CR 406, 305.1, 601.3, 714.2b): the card stays playable for as long as it stays in exile, on each turn you put a lore
/// counter on this Saga, the one this chapter triggered on included. "This Saga" is this object: once it has left the
/// battlefield, no turn qualifies again (CR 400.7).
fn chain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let when = Some(PlayCondition::PutCounterOnThisTurn(item.object, CounterKind::Lore));
    library::exile_top_may_play(g, you, 1, you, PlayGrant { lands: true, when, ..PlayGrant::CAST });
    Ok(())
}

/// "... it gains haste until end of turn" (CR 611.2a, 400.7a): given to the spell on the stack, the effect carries over to
/// the permanent it becomes.
fn it_gains_haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(spell) = item.subject else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, spell, Keyword::Haste);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, chain, "I \u{2014} Chain \u{2014} Exile the top card of your library. During any turn you put a lore counter on this Saga, you may play that card."),
        chapter(2, delayed::watch_for_the_next, "II, III \u{2014} Gestalt Mode \u{2014} When you next cast a creature spell this turn, it gains haste until end of turn."),
        chapter(3, delayed::watch_for_the_next, "II, III \u{2014} Gestalt Mode \u{2014} When you next cast a creature spell this turn, it gains haste until end of turn."),
    ],
    delayed: &[triggered(triggers::CREATURE_SPELL_YOU_CAST, it_gains_haste, "When you next cast a creature spell this turn, it gains haste until end of turn.")],
    ..oracle::SUMMON_BRYNHILDR
};
