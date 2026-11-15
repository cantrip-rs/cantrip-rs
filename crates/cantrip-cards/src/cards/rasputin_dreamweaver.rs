use cantrip_api::*;
use cantrip_mechanics::{effects, prevention, triggers};
use cantrip_oracle as oracle;

/// "If Rasputin started the turn untapped" (CR 603.4), asked as its controller's upkeep begins and again as the ability
/// resolves: whether this permanent was on the battlefield, untapped, as the turn began. One that entered this turn is a new
/// object that didn't start the turn at all (CR 400.7) - an additional upkeep after it entered (Sphinx of the Second Sun,
/// Paradox Haze) asks of it too. Otherwise its first tapping or untapping this turn says what it was then - an untap means it
/// began tapped (`Event::Untapped` is never recorded for an untapped permanent), a tap that it began untapped - and with
/// neither it is as it began. Phasing in during the untap step is not entering (CR 702.26d): the same permanent, tapped or
/// untapped as it was while phased out.
fn started_the_turn_untapped(q: &dyn Query, asker: &Asker) -> bool {
    let me = asker.source;
    let history = q.history();
    if history.entered_this_turn(me) {
        return false;
    }
    let taps = EventQuery::kind(EventMask(EventMask::TAPPED.0 | EventMask::UNTAPPED.0)).object(me).this_turn();
    match history.select(taps).next().map(|r| r.event) {
        Some(Event::Untapped { .. }) => false,
        Some(_) => true,
        None => !q.obj(me).tapped,
    }
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_counters::<{ CounterKind::Dream as usize }, 7>),
    abilities: &[
        mana_ability(remove_counters(CounterKind::Dream, 1), add(Mana::C), "Remove a dream counter from Rasputin: Add {C}."),
        ability(remove_counters(CounterKind::Dream, 1), prevention::next_1_damage_to_this_this_turn,
            "Remove a dream counter from Rasputin: Prevent the next 1 damage that would be dealt to Rasputin this turn."),
        // One past seven comes off again as a state-based action (`counter_ceilings`, CR 704.5r).
        triggered(triggers::YOUR_UPKEEP, effects::counters_on_self::<{ CounterKind::Dream as usize }, 1>,
            "At the beginning of your upkeep, if Rasputin started the turn untapped, put a dream counter on it.")
            .condition(started_the_turn_untapped),
    ],
    granted_preventions: &[prevention::ANY_DAMAGE.text("Prevent the next 1 damage that would be dealt to Rasputin this turn.")],
    counter_ceilings: &[(CounterKind::Dream, 7)],
    ..oracle::RASPUTIN_DREAMWEAVER
};
