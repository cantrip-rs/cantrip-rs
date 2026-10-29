use cantrip_api::*;
use cantrip_mechanics::history::faced_a_dilemma;
use cantrip_mechanics::{choose, select, tokens, top_of_library, triggers};
use cantrip_oracle as oracle;

fn make_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    // Each dilemma faced this turn, asked of the event history (choosing no mode is no dilemma).
    let dilemmas = g.history().this_turn().iter().filter(|r| faced_a_dilemma(&r.event) == Some(player)).count() as u32;
    tokens::create(g, item.controller, &tokens::OFFICER, dilemmas)?;
    Ok(())
}

/// The mana value is the spell's as cast (CR 608.2g, 709.3b, 715.3), so a card is offered only for a face that has it within
/// the number of Officers, and is cast as that face; the cast ones leave the exile, a cast given up leaves its card to be
/// offered again.
fn exile_and_cast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let officer = g.words(item.object).subtype(Subtype::Officer);
    let officers = g.permanents_of(player).into_iter().filter(|&id| g.chars(id).has_subtype(officer)).count() as u32;
    let cards = top_of_library(g, player, 5);
    let exiled = g.exile_all(&cards);
    let within = |w: &CastWay| w.mana_value <= officers;
    loop {
        let mut castable = Vec::new();
        for &c in &exiled {
            if g.obj(c).zone.is(ZoneKind::Exile) && g.cast_ways_during_resolution_by(c, player).iter().any(within) {
                castable.push(c);
            }
        }
        let Some(&card) = choose::select(g, player, item.object, Purpose::CastFree, castable, select::cards(select::up_to(1)))?.first() else {
            return Ok(());
        };
        let ways: Vec<CastWay> = g.cast_ways_during_resolution_by(card, player).into_iter().filter(within).collect();
        let way = choose::cast_way(g, player, card, ways)?.map(|w| w.way);
        g.cast_during_resolution_as(card, CastTerms { instead: Some(ManaCost::ZERO), way, caster: Some(player), ..CastTerms::default() })?;
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::YOUR_END_STEP, make_token, "At the beginning of your end step, create a 1/1 red Officer creature token for each dilemma you've faced this turn."),
        ability(cost("{2}{W}{U}{B}{R}{G}").and(TAP), exile_and_cast, "{2}{W}{U}{B}{R}{G}, {T}: Exile the top five cards of your library. You may cast any number of spells with mana value less than or equal to the number of Officers you control from among those cards without paying their mana costs.")
    ],
    ..oracle::CAPTAIN_BENJAMIN_SISKO
};

pub static EXTRA: &[&CardDef] = &[&tokens::OFFICER];
