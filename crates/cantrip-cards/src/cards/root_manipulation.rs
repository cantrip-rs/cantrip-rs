use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// Until end of turn, the creatures you control as it resolves get +2/+2 and gain menace and the attack trigger (CR 611.2c, 613.1f):
/// no creature that enters later gets any of it.
fn pump_and_grant(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let this = g.obj(item.object).card;
    instruction(g, |g| {
        for creature in effects::pump_creatures_you_control(g, item, 2, 2) {
            effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Menace);
            g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], pump_and_grant),
    // The granted ability (the card's delayed ability 0).
    delayed: &[triggered(triggers::THIS_ATTACKS, effects::gain_one_life, "Whenever this creature attacks, you gain 1 life.")],
    ..oracle::ROOT_MANIPULATION
};
