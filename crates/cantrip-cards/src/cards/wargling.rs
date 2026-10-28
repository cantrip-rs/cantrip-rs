use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{ferocious, pump, trample_until_end_of_turn};
use cantrip_oracle as oracle;

/// "Until end of turn, this creature gets +1/+0 and creatures you control gain trample" (CR 611.2a, 611.2c): one instruction; the
/// creatures that gain trample are those its controller controls as it resolves, and none that enters later.
fn pump_and_trample(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        pump(g, item.object, item.object, 1, 0);
        for &id in &creatures {
            trample_until_end_of_turn(g, item.object, id);
        }
    });
    Ok(())
}

/// "...while you control a creature with power 4 or greater": asked as the attack is declared, as part of the trigger event, and
/// never again as the ability resolves (this is no intervening "if", CR 603.4; the ruling).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        pump_and_trample,
        "Ferocious \u{2014} Whenever this creature attacks while you control a creature with power 4 or greater, until end of turn, this creature gets +1/+0 and creatures you control gain trample.",
    )
    .only_if(ferocious::you_control_asked)],
    ..oracle::WARGLING
};
