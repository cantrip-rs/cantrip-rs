use cantrip_api::*;
use cantrip_mechanics::{boast, choose, effects};
use cantrip_oracle as oracle;

/// "Whenever you cast a black spell from your hand" (CR 601.2a, 603.2): the spell is its controller's, black as it is on the stack
/// (CR 601.2i), and the zone it was cast from, recorded on it, is the hand.
fn you_cast_a_black_spell_from_your_hand(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. }
            if *player == me.controller && q.obj(*spell).cast_from == Some(ZoneKind::Hand) && q.chars(*spell).colors.has(q.words(me.source).color(Color::Black)) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Copy those exiled cards. You may cast up to three of the copies without paying their mana costs" (CR 707.12, 608.2g): the
/// cards this activation's cost exiled (`Query::cost_objects`, CR 602.2b), those still in exile as it resolves, each cast as a copy
/// of the exiled card, which a permanent spell resolves into as a token (CR 707.10f) and which ceases to exist if it is not cast
/// (CR 704.5e). Chosen one at a time, and a copy is cast once: the card is left in exile, but its copy is spent.
fn copy_the_exiled_cards_and_cast_up_to_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let mut found: Vec<ObjectId> = g.cost_objects(item).iter().copied().filter(|&c| is_live(g, c)).collect();
    let mut cast = 0;
    while cast < 3 {
        let mut castable: Vec<ObjectId> = Vec::new();
        for &c in &found {
            if !g.cast_ways_during_resolution(c).is_empty() {
                castable.push(c);
            }
        }
        let rule = Selection::listed(Count::UpTo(1), &found, castable);
        let chosen = choose::select(g, you, item.object, Purpose::CastCopyFree, found.clone(), rule)?;
        let Some(&card) = chosen.first() else { return Ok(()) };
        // The face is chosen before the spell is put onto the stack (CR 709.3), asked only when there is a choice.
        let ways = g.cast_ways_during_resolution(card);
        let Some(way) = choose::cast_way(g, you, card, ways)? else { return Ok(()) };
        let terms = CastTerms { instead: Some(ManaCost::ZERO), copy: true, way: Some(way.way), ..CastTerms::default() };
        // A cast the player gives up is undone (CR 733.1): the copy is not cast and counts for nothing, and they may choose again.
        if g.cast_during_resolution_as(card, terms)? {
            found.retain(|&c| c != card);
            cast += 1;
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, you_cast_a_black_spell_from_your_hand),
            effects::this_connives,
            "Whenever you cast a black spell from your hand, Baron Helmut Zemo connives. (Draw a card, then discard a card. If you discarded a nonland card, put a +1/+1 counter on this creature.)",
        ),
        boast::ability(
            "Boast \u{2014} Exile any number of black cards from your graveyard with fifteen or more black mana symbols among their mana costs: Copy those exiled cards. You may cast up to three of the copies without paying their mana costs. (Activate only if this creature attacked this turn and only once each turn.)",
            exile_from_graveyard_with_symbols(Color::Black, 15),
            &[],
            copy_the_exiled_cards_and_cast_up_to_three,
        ),
    ],
    ..oracle::BARON_HELMUT_ZEMO
};
