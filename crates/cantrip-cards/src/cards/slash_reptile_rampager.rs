use cantrip_api::*;
use cantrip_mechanics::triggers::{ANOTHER_CREATURE_YOU_CONTROL_ENTERS, THIS_ATTACKS};
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Slash deals 2 damage to each opponent."
fn damage_opponents(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent(g, item, 2);
    Ok(())
}

/// Alliance (the rulings of Mighty Mutanimals): the Mutant token entering is another creature entering under its
/// controller's control, so it triggers the first ability once the second has resolved.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
            damage_opponents,
            "Alliance \u{2014} Whenever another creature you control enters, Slash deals 2 damage to each opponent.",
        ),
        triggered(THIS_ATTACKS, tokens::create_a_mutant, "Whenever Slash attacks, create a 2/2 red Mutant creature token."),
    ],
    ..oracle::SLASH_REPTILE_RAMPAGER
};

pub static EXTRA: &[&CardDef] = &[&tokens::MUTANT];
