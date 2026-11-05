use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{counters_divided, craft, targets};
use cantrip_oracle as oracle;

/// "Distribute three +1/+1 counters among one, two, or three target creatures you control." The targets and their shares
/// are fixed as the ability goes on the stack, each target getting at least one (CR 601.2d, 603.3d); a target illegal as
/// it resolves takes its share with it (CR 608.2b, the card's ruling).
fn distribute(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counters_divided(g, item, CounterKind::PlusOne);
    Ok(())
}

/// "You gain life equal to the mana value of the exiled card used to craft it" (CR 702.167c): the cards the craft cost
/// exiled and still in exile, their mana values added up when there are several (the card's ruling); none for a token,
/// which ceased to exist, or for a Jadeheart Attendant that entered any other way.
fn gain_crafted_mana_value(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let total: u32 = craft::exiled_cards_used_to_craft(g, item.object).into_iter().map(|c| g.mana_value(c)).sum();
    g.gain_life(item.controller, i64::from(total as i32));
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, gain_crafted_mana_value, "When this creature enters, you gain life equal to the mana value of the exiled card used to craft it.")],
    ..oracle::JADEHEART_ATTENDANT
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, distribute, "When this artifact enters, distribute three +1/+1 counters among one, two, or three target creatures you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL.between(1, 3)])
            .divide(Divide { total: 3, of_x: None }),
        craft::ability(mana("{5}{G}{G}"), &craft::CREATURE, "Craft with creature {5}{G}{G}"),
    ],
    back: Some(&BACK),
    ..oracle::JADE_SEEDSTONES
};
