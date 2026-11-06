use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{craft, effects, targets};
use cantrip_oracle as oracle;

/// "You gain life equal to the mana value of the exiled card used to craft it" (CR 702.167c): the cards the craft cost
/// exiled and still in exile, their mana values added up when there are several (the card's ruling); none for a token,
/// which ceased to exist, or for a Jadeheart Attendant that entered any other way.
fn gain_crafted_mana_value(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let total: u32 = craft::exiled_cards_used_to_craft(g, item.object).into_iter().map(|c| g.mana_value(c)).sum();
    g.gain_life(item.controller,(total as i32) as i32);
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, gain_crafted_mana_value, "When this creature enters, you gain life equal to the mana value of the exiled card used to craft it.")],
    ..oracle::JADEHEART_ATTENDANT
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::distribute_plus_one_counters, "When this artifact enters, distribute three +1/+1 counters among one, two, or three target creatures you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL.between(1, 3)])
            .divide(Divide { total: 3, reads: None }),
        craft::ability(mana("{5}{G}{G}"), &craft::CREATURE, "Craft with creature {5}{G}{G}"),
    ],
    back: Some(&BACK),
    ..oracle::JADE_SEEDSTONES
};
