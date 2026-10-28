use cantrip_api::*;
use cantrip_mechanics::triggers::{ward, YOU_ATTACK};
use cantrip_mechanics::{choose, library};
use cantrip_oracle as oracle;

/// The greatest power among the attacking creatures `player` controls, read as the ability resolves (CR 608.2h): none if
/// none is attacking any more, and then no spell has a mana value "less than or equal to" it.
fn greatest_attacking_power(g: &dyn Game, player: PlayerId) -> Option<i16> {
    g.permanents_of(player).into_iter().filter(|&id| g.is_attacking(id)).filter_map(|id| g.chars(id).power).max()
}

/// "Whenever you attack, look at the top six cards of your library. You may cast a spell from among them with mana value
/// less than or equal to the greatest power among attacking creatures you control without paying its mana cost. Put the
/// rest on the bottom of your library in a random order." The mana value is the spell's (CR 608.2g, 709.3b), so a card is
/// offered only for a face that has it within the limit; the cards stay in the library until cast.
fn attack(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let looked = cantrip_mechanics::top_of_library(g, you, 6);
    if looked.is_empty() {
        return Ok(());
    }
    g.look_at(you, &looked);
    let limit = greatest_attacking_power(g, you);
    let within = |w: &CastWay| limit.is_some_and(|p| i64::from(w.mana_value) <= i64::from(p));
    let mut castable = Vec::new();
    for &c in &looked {
        if g.cast_ways_during_resolution(c).iter().any(within) {
            castable.push(c);
        }
    }
    if let Some(&card) = choose::cards_from(g, you, item.object, Purpose::CastFree, looked.clone(), castable, 1, true)?.first() {
        let ways: Vec<CastWay> = g.cast_ways_during_resolution(card).into_iter().filter(within).collect();
        if let Some(way) = choose::cast_way(g, you, card, ways)? {
            g.cast_during_resolution_as(card, CastTerms { instead: Some(ManaCost::ZERO), way: Some(way.way), ..CastTerms::default() })?;
        }
    }
    library::rest_to_bottom_in_random_order(g, you, &looked);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<2>("Ward {2}"),
        triggered(
            YOU_ATTACK,
            attack,
            "Whenever you attack, look at the top six cards of your library. You may cast a spell from among them with mana value less than or equal to the greatest power among attacking creatures you control without paying its mana cost. Put the rest on the bottom of your library in a random order.",
        ),
    ],
    ..oracle::COSMIC_CUBE
};
