use cantrip_api::*;
use cantrip_mechanics::triggers::EACH_BEGINNING_OF_COMBAT;
use cantrip_mechanics::{choose, delayed, fight, library, select, targets, top_of_library};
use cantrip_oracle as oracle;

/// "Look at the top five cards of your library. You may exile a creature card from among them. Put the rest on the bottom of your
/// library in a random order. You may cast the exiled card this turn. At the beginning of the next combat phase this turn, target
/// creature you control deals damage equal to its power to up to one target creature you don't control." (CR 701.13a, 401.4,
/// 601.3, 603.7b, 603.3d): the cards looked at are all shown and the creature card is chosen as the spell resolves, declining being the
/// empty answer; the rest of the cards go to the bottom in a random order. The exiled card, if it reached exile, may be cast
/// until the turn ends. The delayed ability is made whatever was exiled and takes its targets as it goes on the stack.
fn look_exile_a_creature_may_cast_it_then_bite_at_combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let looked = top_of_library(g, you, 5);
    g.look_at(you, &looked);
    let chosen = choose::select(g, you, item.object, Purpose::Exile, looked.clone(), select::CREATURE_CARD)?;
    let exiled = g.exile_all(&chosen);
    library::rest_to_bottom_in_random_order(g, you, &looked);
    let grant = PlayGrant { expiry: Expiry::EndOfTurn, ..PlayGrant::CAST };
    for card in exiled {
        if g.obj(card).zone.is(ZoneKind::Exile) {
            g.allow_play(card, you, grant);
        }
    }
    delayed::next_this_turn(g, item, 0, item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], look_exile_a_creature_may_cast_it_then_bite_at_combat),
    delayed: &[triggered(
        EACH_BEGINNING_OF_COMBAT,
        fight::first_target_bites_second,
        "At the beginning of the next combat phase this turn, target creature you control deals damage equal to its power to up to one target creature you don't control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_YOU_DONT_CONTROL.optional()])],
    ..oracle::FERAL_ENCOUNTER
};
