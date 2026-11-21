use cantrip_api::*;
use cantrip_mechanics::{library, select, tokens, triggers};
use cantrip_oracle as oracle;

/// A 4/4 green Bear creature token (CR 111.4, 205.3m).
pub static BEAR: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    ..token!(Bear)
};

/// "Whenever one or more creatures you control with power 4 or greater attack" (CR 508.1m, 603.2c): a creature its controller
/// declares as an attacker whose power is 4 or more as the attack is declared (a creature that grows afterwards does not trigger it). The
/// attackers declared together are one batch (`once_per_batch`), each a subject.
fn you_attack_with_a_creature_with_power_4_or_greater(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if q.controller(*attacker) == me.controller && q.chars(*attacker).power.is_some_and(|power| power >= 4) => {
            Fired::about(Target::Object(*attacker))
        }
        _ => None,
    }
}

/// "Search your library for up to that many basic land cards, put them onto the battlefield tapped, then shuffle" (CR 701.23): that
/// many is the number of creatures with power 4 or greater that attacked together, which the batch holds; the library is shuffled
/// even if none is found.
fn search_for_that_many_basic_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = g.event_objects(item).len() as u32;
    library::find_to_battlefield(g, item.controller, item.object, select::basic_land_cards(select::up_to(count)), true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, |g, item| tokens::create(g, item.controller, &BEAR, 1).map(drop), "When The Earth King enters, create a 4/4 green Bear creature token."),
        triggered(
            when_asked(EventMask::ATTACKS, you_attack_with_a_creature_with_power_4_or_greater),
            search_for_that_many_basic_lands,
            "Whenever one or more creatures you control with power 4 or greater attack, search your library for up to that many basic land cards, put them onto the battlefield tapped, then shuffle.",
        )
        .once_per_batch(),
    ],
    ..oracle::THE_EARTH_KING
};

pub static EXTRA: &[&CardDef] = &[&BEAR];
