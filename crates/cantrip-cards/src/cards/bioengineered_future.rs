use cantrip_api::*;
use cantrip_mechanics::tokens::{self, LANDER};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// How many lands have entered the battlefield under `player`'s control this turn (CR 603.6a), judged as each entered (the
/// controller and card types `Event::Entered` holds), so one that has since left or been stolen still counts.
fn lands_entered_under_control_this_turn(q: &dyn Query, player: PlayerId) -> i64 {
    q.history().entered().filter(|e| e.controller == player && e.is(CardType::Land)).count().try_into().expect("land entry count fits i64")
}

/// "Each creature you control enters with an additional +1/+1 counter on it for each land that entered the battlefield under
/// your control this turn" (CR 614.1c, 122.6): a creature about to enter the battlefield under this permanent's controller, card
/// or token alike, while at least one land has entered under their control this turn - a creature entering at the same time as a
/// land has not seen it enter yet (CR 614.12).
fn a_creature_enters_under_you_after_a_land(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.to.is(ZoneKind::Battlefield) && z.controller == asker.controller && q.chars(z.object).is(CardType::Creature))
        && lands_entered_under_control_this_turn(q, asker.controller) > 0
}

fn add_a_counter_for_each_land(q: &dyn Query, asker: &Asker, proposal: &mut Proposal) {
    let more = lands_entered_under_control_this_turn(q, asker.controller);
    if let Proposal::ZoneChange(z) = proposal {
        z.counters[CounterKind::PlusOne as usize] += more;
    }
}

/// "When this enchantment enters, create a Lander token." (CR 111.10u)
fn lander(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_lander(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, lander, "When this enchantment enters, create a Lander token.")],
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        a_creature_enters_under_you_after_a_land,
        add_a_counter_for_each_land,
        "puts an additional +1/+1 counter on it for each land that entered under your control this turn",
    )],
    ..oracle::BIOENGINEERED_FUTURE
};

pub static EXTRA: &[&CardDef] = &[&LANDER];
