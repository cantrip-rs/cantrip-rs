use cantrip_api::*;
use cantrip_mechanics::statics::{have_haste, other_of_subtype_you_control};
use cantrip_mechanics::triggers::{BEGINNING_OF_YOUR_COMBAT, THIS_ENTERS};
use cantrip_mechanics::{lands, sacrifice_one_of, tokens};
use cantrip_oracle as oracle;

/// The 0/1 green Dinosaur Egg creature token (CR 111.1, 111.4): Palani's Hatcher makes it.
pub static DINOSAUR_EGG: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(1),
    ..token!(Dinosaur Egg)
};

/// "Create two 0/1 green Dinosaur Egg creature tokens" (CR 111.1, 701.7): both made together, as one event.
fn create_two_eggs(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &DINOSAUR_EGG, 2)?;
    Ok(())
}

/// The Eggs `controller` controls, "Egg" read through the words of the source's text (CR 612.2): any creature with the creature
/// type, not only the tokens this card makes (the ruling).
fn eggs(q: &dyn Query, source: ObjectId, controller: PlayerId) -> Vec<ObjectId> {
    let egg = q.words(source).subtype(Subtype::Egg);
    q.permanents_of(controller)
        .into_iter()
        .filter(|&id| q.chars(id).has_subtype(egg))
        .collect()
}

/// "If you control one or more Eggs" (CR 603.4): an intervening if, asked as the ability triggers and again as it resolves.
fn you_control_an_egg(q: &dyn Query, asker: &Asker) -> bool {
    !eggs(q, asker.source, asker.controller).is_empty()
}

/// "Sacrifice an Egg, then create a 3/3 green Dinosaur creature token" (CR 701.21a, 608.2c): the Egg is the controller's choice among
/// those they control, and the Dinosaur comes whether or not the sacrifice happened.
fn sacrifice_an_egg_then_make_a_dinosaur(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let eggs = eggs(g, item.object, item.controller);
    sacrifice_one_of(g, item.controller, item.object, 1, eggs)?;
    tokens::create(g, item.controller, &tokens::GREEN_DINOSAUR, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, create_two_eggs, "When this creature enters, create two 0/1 green Dinosaur Egg creature tokens."),
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            sacrifice_an_egg_then_make_a_dinosaur,
            "At the beginning of combat on your turn, if you control one or more Eggs, sacrifice an Egg, then create a 3/3 green Dinosaur creature token.",
        )
        .condition(you_control_an_egg),
    ],
    statics: &[static_ability(
        Layer::Ability,
        other_of_subtype_you_control::<{ lands::sub(Subtype::Dinosaur) }>,
        have_haste,
        "Other Dinosaurs you control have haste.",
    )],
    ..oracle::PALANIS_HATCHER
};

pub static EXTRA: &[&CardDef] = &[&DINOSAUR_EGG, &tokens::GREEN_DINOSAUR];
