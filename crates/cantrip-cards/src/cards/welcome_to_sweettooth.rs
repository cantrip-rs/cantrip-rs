use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "I — Create a 1/1 white Human creature token." (CR 111.1)
fn create_human(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::HUMAN, 1)?;
    Ok(())
}

/// "III — Put X +1/+1 counters on target creature you control, where X is one plus the number of Foods you control." (CR 122.1a,
/// 608.2b): X is counted as the chapter resolves, the Foods under the Saga's controller's control, tokens and cards alike. With
/// its target illegal it does nothing.
fn put_one_plus_foods_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let foods = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).has_subtype(Subtype::Food)).count() as i32;
    g.add_counters(creature, CounterKind::PlusOne, 1 + foods);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, create_human, "I \u{2014} Create a 1/1 white Human creature token."),
        chapter(2, tokens::create_a_food_token, "II \u{2014} Create a Food token."),
        chapter(3, put_one_plus_foods_counters, "III \u{2014} Put X +1/+1 counters on target creature you control, where X is one plus the number of Foods you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    ..oracle::WELCOME_TO_SWEETTOOTH
};

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN, &tokens::FOOD];
