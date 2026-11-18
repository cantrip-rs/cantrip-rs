use cantrip_api::*;
use cantrip_mechanics::disguise;
use cantrip_oracle as oracle;

/// "The number of other creatures you control" (CR 700.7): `you` controls them now, `this` not among them.
fn other_creatures_you_control(g: &dyn Game, you: PlayerId, this: ObjectId) -> i32 {
    g.permanents_of(you).into_iter().filter(|&id| id != this && g.chars(id).is(CardType::Creature)).count() as i32
}

/// "As this creature enters, put X +1/+1 counters on it, where X is the number of other creatures you control" (CR 614.1c,
/// 122.1): read as it enters, when it is not yet on the battlefield. Not read as it enters face down (CR 708.2): a face-down
/// permanent has no abilities.
fn enters_with_a_counter_for_each_other_creature(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[CounterKind::PlusOne as usize] += other_creatures_you_control(g, entering.controller, entering.object);
    Ok(())
}

/// "As this creature is turned face up, put X +1/+1 counters on it, where X is the number of other creatures you control"
/// (CR 708.11): put on it as it is turned up, so what triggers on its being turned face up sees them.
fn turned_face_up_with_a_counter_for_each_other_creature(g: &mut dyn Game, asker: &Asker) {
    let x = other_creatures_you_control(g, asker.controller, asker.source);
    g.add_counters(asker.source, CounterKind::PlusOne, x);
}

pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{3}{G/W}{G/W}")), "Disguise {3}{G/W}{G/W}")],
    enters: Some(enters_with_a_counter_for_each_other_creature),
    as_turned_face_up: Some(turned_face_up_with_a_counter_for_each_other_creature),
    ..oracle::CROWD_CONTROL_WARDEN
};
