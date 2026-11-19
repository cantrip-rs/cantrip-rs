use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Target creature gets +2/+0 and gains first strike until end of turn. / Create a Treasure token." Two sentences (CR 608.2c): the
/// spell's one target is checked as it resolves, and if it is illegal the spell does not resolve and no Treasure is made (the ruling).
fn pump_then_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gets_and_gains_first_strike::<2, 0>(g, item)?;
    tokens::create_a_treasure(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], pump_then_treasure),
    ..oracle::ANCESTORS_AID
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
