use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, evoke, pump, spent, tokens};
use cantrip_oracle as oracle;

/// "Create two 1/1 green and white Kithkin creature tokens."
fn two_kithkin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::KITHKIN, 2)?;
    Ok(())
}

/// "Creatures you control get +1/+1 and gain haste until end of turn" (CR 611.2c): the creatures you control as it
/// resolves, and no creature that enters later.
fn team_pump(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<ObjectId> =
        g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for creature in creatures {
            pump(g, item.object, creature, 1, 1);
            effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Haste);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    ways: &[evoke::way(Cost::mana(mana("{R/W}{R/W}")), "Evoke {R/W}{R/W}")],
    abilities: &[
        evoke::sacrifice_trigger("When Catharsis enters, if its evoke cost was paid, its controller sacrifices it."),
        triggered(THIS_ENTERS, two_kithkin, "When this creature enters, if {W}{W} was spent to cast it, create two 1/1 green and white Kithkin creature tokens.")
            .condition(spent::two::<{ Mana::W as u8 }>),
        triggered(THIS_ENTERS, team_pump, "When this creature enters, if {R}{R} was spent to cast it, creatures you control get +1/+1 and gain haste until end of turn.")
            .condition(spent::two::<{ Mana::R as u8 }>),
    ],
    ..oracle::CATHARSIS
};

pub static EXTRA: &[&CardDef] = &[&tokens::KITHKIN];
