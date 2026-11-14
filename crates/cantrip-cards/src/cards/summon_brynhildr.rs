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

/// II, III: "When you next cast a creature spell this turn, ..." (CR 603.7b): the chapter sets up the delayed trigger.
fn gestalt_mode(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::next_this_turn(g, item, 0, item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, chain, "I \u{2014} Chain \u{2014} Exile the top card of your library. During any turn you put a lore counter on this Saga, you may play that card."),
        chapter(2, gestalt_mode, "II, III \u{2014} Gestalt Mode \u{2014} When you next cast a creature spell this turn, it gains haste until end of turn."),
        chapter(3, gestalt_mode, "II, III \u{2014} Gestalt Mode \u{2014} When you next cast a creature spell this turn, it gains haste until end of turn."),
    ],
    delayed: &[triggered(triggers::CREATURE_SPELL_YOU_CAST, effects::subject_gains_haste_until_end_of_turn, "When you next cast a creature spell this turn, it gains haste until end of turn.")],
    ..oracle::SUMMON_BRYNHILDR
};
