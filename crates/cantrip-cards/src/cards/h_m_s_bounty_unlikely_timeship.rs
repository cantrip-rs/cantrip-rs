use cantrip_api::*;
use cantrip_mechanics::{library, select, statics, station, triggers};
use cantrip_oracle as oracle;

const SEARCH: Question = Question::new(
    "SearchLibrary",
    "search your library for a Whale card?",
    "Reveal it and put it into your hand, then shuffle",
    "Search",
    "Don't search",
);

fn whale_card(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let whale = q.words(choice.source).subtype(Subtype::Whale);
    Verdict::each(choice, |card| q.chars(card).has_subtype(whale))
}

/// "You may search your library for a Whale card, reveal it, put it into your hand, then shuffle." (CR 701.20a, 701.23.)
fn search_for_whale(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !library::ask_may_search(g, item.controller, item.object, &SEARCH)? {
        return Ok(());
    }
    let rule = select::Selection::new(select::up_to(1))
        .called("Whale card")
        .with(select::part(&(whale_card as SelectFn), 0, ""));
    library::find_to_hand(g, item.controller, item.object, rule, true)?;
    Ok(())
}

/// "If you control a Whale and haven't begun an extra turn this game, take an extra turn after this one." (CR 500.7.)
fn combat_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    let whale = g.words(item.object).subtype(Subtype::Whale);
    let controls_whale = g
        .permanents_of(item.controller)
        .into_iter()
        .any(|permanent| g.chars(permanent).has_subtype(whale));
    let began_extra_turn = g
        .history()
        .select(EventQuery::kind(EventMask::TURN_BEGAN).player(item.controller))
        .any(|record| matches!(record.event, Event::TurnBegan { extra: true, .. }));
    if controls_whale && !began_extra_turn {
        g.add_extra_turns(item.controller, 1);
    }
    Ok(())
}

/// The 8+ striation: a 3/4 artifact creature with flying (CR 721.2b).
const EIGHT_PLUS: &str = "8+ | Flying (3/4)";

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            search_for_whale,
            "When H.M.S. Bounty enters, you may search your library for a Whale card, reveal it, put it into your hand, then shuffle.",
        ),
        station::ability(
            "Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)",
        ),
    ],
    statics: &[
        static_ability(
            Layer::Ability,
            station::charged::<8>,
            station::has::<0>,
            EIGHT_PLUS,
        ),
        static_ability(
            Layer::Type,
            station::charged::<8>,
            station::is_a_creature,
            EIGHT_PLUS,
        ),
        static_ability(
            Layer::Ability,
            station::charged::<8>,
            statics::have_flying,
            EIGHT_PLUS,
        )
        .continues(),
        static_ability(
            Layer::PtSet,
            station::charged::<8>,
            statics::base_pt::<3, 4>,
            EIGHT_PLUS,
        )
        .continues(),
    ],
    // The 8+ striation (CR 721.2a): a triggered ability Bounty has while it has eight or more charge counters.
    delayed: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER,
        combat_damage,
        "Whenever H.M.S. Bounty deals combat damage to a player, draw a card. If you control a Whale and haven't begun an extra turn this game, take an extra turn after this one.",
    )],
    ..oracle::H_M_S_BOUNTY_UNLIKELY_TIMESHIP
};
