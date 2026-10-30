use cantrip_api::*;
use cantrip_mechanics::history::you_gained_life_this_turn;
use cantrip_mechanics::{discover, effects, library, targets, triggers};
use cantrip_oracle as oracle;

/// "Another target creature you control gains flying and lifelink until end of turn."
fn lend_lifelink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Flying);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Lifelink);
    });
    Ok(())
}

/// "Exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its
/// mana cost if the spell's mana value is less than or equal to the amount of life you gained this turn. Otherwise, put it
/// into your hand." The life is counted as the ability resolves; the lands exiled on the way stay in exile. The spell's
/// mana value is that of the face it is cast as (CR 601.2b), as Discover reads it (CR 701.57a).
fn exile_and_cast_or_take(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let gained = g.history().life_gained(you);
    let (_, hit) = library::exile_until_nonland(g, you, None, &|_, _| true);
    let Some(card) = hit else { return Ok(()) };
    discover::cast_free_if_at_most_else_hand(g, you, card, gained)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}{W}").and(TAP), lend_lifelink, "{1}{W}, {T}: Another target creature you control gains flying and lifelink until end of turn.")
            .targets(&[targets::OTHER_CREATURE_YOU_CONTROL]),
        triggered(
            triggers::YOUR_END_STEP,
            exile_and_cast_or_take,
            "At the beginning of your end step, if you gained life this turn, exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana cost if the spell's mana value is less than or equal to the amount of life you gained this turn. Otherwise, put it into your hand.",
        )
        .condition(you_gained_life_this_turn),
    ],
    ..oracle::BRE_OF_CLAN_STOUTARM
};
