use cantrip_api::*;
use cantrip_mechanics::{effects, flashback, lands, tokens};
use cantrip_oracle as oracle;

/// "If this spell was cast from anywhere other than your hand": asked of how it was cast (CR 601.2a), the zone its cast record
/// names and whose hand that was, a card in a hand being in its owner's. A copy an effect makes was never cast (CR 707.10), so
/// it was not cast from anywhere and the condition is false; a copy that is cast has a cast record like any spell.
fn cast_from_other_than_your_hand(g: &dyn Game, item: &StackItem) -> bool {
    g.history().cast_of(item.object).is_some_and(|c| c.from != ZoneKind::Hand || g.obj(item.object).owner != c.player)
}

/// "Create two 2/2 red and white Spirit creature tokens. Then if this spell was cast from anywhere other than your hand, put a
/// +1/+1 counter on each Spirit you control": the Spirits it has just made and every other, at once (CR 608.2, 101.4).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::SPIRIT_2_2_RED_WHITE, 2)?;
    g.end_sentence();
    if cast_from_other_than_your_hand(g, item) {
        effects::plus_one_counter_on_each_of_subtype_you_control::<{ lands::sub(Subtype::Spirit) }>(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[flashback::way(cost("{4}{W}{W}"), "Flashback {4}{W}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)")],
    ..oracle::ANTIQUITIES_ON_THE_LOOSE
};

pub static EXTRA: &[&CardDef] = &[&tokens::SPIRIT_2_2_RED_WHITE];
