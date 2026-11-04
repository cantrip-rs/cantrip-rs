use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Deafening Clarion deals 3 damage to each creature." (CR 120.3, 608.2c).
fn three_damage_to_each_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_creature(g, item, 3);
    Ok(())
}

/// "Creatures you control gain lifelink until end of turn." (CR 611.2c, 702.15).
fn creatures_you_control_gain_lifelink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::creatures_you_control_gain(g, item, &[Keyword::Lifelink]);
    Ok(())
}

static MODES: ModalDef = modes(&[
    mode(three_damage_to_each_creature, "Deafening Clarion deals 3 damage to each creature."),
    mode(creatures_you_control_gain_lifelink, "Creatures you control gain lifelink until end of turn."),
])
.choose(1, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODES), ..oracle::DEAFENING_CLARION };
