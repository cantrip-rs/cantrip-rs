use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::YOUR_FIRST_MAIN_PHASE;
use cantrip_oracle as oracle;

/// The Adventure (CR 715), "Easy Pickings deals 1 damage to each creature your opponents control."
fn easy_pickings(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_creature_opponents_control(g, item, 1);
    Ok(())
}

pub static EASY_PICKINGS: CardDef = CardDef { spell: spell(&[], easy_pickings), ..oracle::EASY_PICKINGS };

/// "At the beginning of your first main phase, add {R}{R}" (CR 106.4, 505.1a): the mana empties at the end of that phase.
fn add_red_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::R, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOUR_FIRST_MAIN_PHASE, add_red_red, "At the beginning of your first main phase, add {R}{R}.")],
    adventure: Some(&EASY_PICKINGS),
    ..oracle::GL_IN_THE_MIGHTY
};

pub static EXTRA: &[&CardDef] = &[&EASY_PICKINGS];
