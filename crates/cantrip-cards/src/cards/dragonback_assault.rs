use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

/// "It deals 3 damage to each creature and each planeswalker" (CR 120.3): all together, the enchantment itself the source.
fn damage_each_creature_and_planeswalker(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_creature_and_planeswalker(g, item, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // Landfall is an ability word (CR 207.2c): the trigger is an ordinary one, whenever a land enters under your control.
    abilities: &[
        triggered(triggers::THIS_ENTERS, damage_each_creature_and_planeswalker, "When this enchantment enters, it deals 3 damage to each creature and each planeswalker."),
        triggered(triggers::LANDFALL, tokens::create_a_dragon_4_4, "Landfall \u{2014} Whenever a land you control enters, create a 4/4 red Dragon creature token with flying."),
    ],
    ..oracle::DRAGONBACK_ASSAULT
};

pub static EXTRA: &[&CardDef] = &[&tokens::DRAGON_4_4];
