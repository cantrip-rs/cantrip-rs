use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("BecomeBasePower", "have this creature's base power become equal to {subject}'s power until end of turn?", "", "Become equal", "Don't");

/// "You may have this creature's base power become equal to that creature's power until end of turn" (CR 613.4b, 611.2a): the
/// entering Dinosaur's power is read as the ability resolves (the power it last had on the battlefield if it has left, the
/// rulings), and this creature's base power is set to that number for the turn, a layer 7b effect that leaves its toughness,
/// counters and pumps alone (CR 613.4c).
fn base_power_becomes_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(entering) = item.subject else { return Ok(()) };
    let power = targets::power_or_last_known(g, entering);
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        g.add_effect(item.object, item.object, EffectKind::SetPower(power), Duration::EndOfTurn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Dinosaur) }>),
        base_power_becomes_its_power,
        "Whenever another Dinosaur you control enters, you may have this creature's base power become equal to that creature's power until end of turn.",
    )],
    ..oracle::BELLIGERENT_YEARLING
};
