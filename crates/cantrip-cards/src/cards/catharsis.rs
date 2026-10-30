use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, evoke, spent, tokens};
use cantrip_oracle as oracle;

/// "Create two 1/1 green and white Kithkin creature tokens."
fn two_kithkin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::KITHKIN, 2)?;
    Ok(())
}

/// "Creatures you control get +1/+1 and gain haste until end of turn."
fn team_pump(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_creatures_you_control_and_gain(g, item, 1, 1, Keyword::Haste);
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
