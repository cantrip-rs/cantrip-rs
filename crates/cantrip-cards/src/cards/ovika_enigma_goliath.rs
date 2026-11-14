use cantrip_api::*;
use cantrip_mechanics::{haste_until_end_of_turn, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create X 1/1 red Phyrexian Goblin creature tokens, where X is the mana value of that spell. They gain haste until end of
/// turn": X is the spell's mana value as the ability resolves, or as it last was on the stack if it has left (CR 608.2h), an
/// {X} in its cost counting as the X announced (CR 202.3e); "they" are the tokens this ability made, and no others (CR 611.2c).
fn goblins_for_the_spells_mana_value(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(spell) = item.subject else { return Ok(()) };
    let amount = g.mana_value(spell);
    let made = tokens::create(g, item.controller, &tokens::PHYREXIAN_GOBLIN, amount)?;
    g.end_sentence();
    instruction(g, |g| {
        for goblin in made {
            haste_until_end_of_turn(g, item.object, goblin);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward_mana_and_life::<3, 3>("Ward\u{2014}{3}, Pay 3 life. (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {3} and 3 life.)"),
        triggered(
            triggers::NONCREATURE_SPELL_YOU_CAST_ABOUT_IT,
            goblins_for_the_spells_mana_value,
            "Whenever you cast a noncreature spell, create X 1/1 red Phyrexian Goblin creature tokens, where X is the mana value of that spell. They gain haste until end of turn.",
        ),
    ],
    ..oracle::OVIKA_ENIGMA_GOLIATH
};

pub static EXTRA: &[&CardDef] = &[&tokens::PHYREXIAN_GOBLIN];
