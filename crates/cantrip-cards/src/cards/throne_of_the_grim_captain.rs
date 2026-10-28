use cantrip_api::*;
use cantrip_mechanics::combat::choose_attack_target;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{choose, craft, each_opponent_sacrifices};
use cantrip_oracle as oracle;

/// "{T}: Mill two cards." (CR 701.17)
fn mill_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 2);
    Ok(())
}

/// A material of a creature type named alone: a permanent of that type or a card of it in the graveyard (CR 702.167b), the
/// type read through the Throne's words (CR 612).
fn of_type(q: &dyn Query, asker: &Asker, id: ObjectId, subtype: Subtype) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(subtype))
}

fn dinosaur(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    of_type(q, asker, id, Subtype::Dinosaur)
}

fn merfolk(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    of_type(q, asker, id, Subtype::Merfolk)
}

fn pirate(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    of_type(q, asker, id, Subtype::Pirate)
}

fn vampire(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    of_type(q, asker, id, Subtype::Vampire)
}

fn any_of_them(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    dinosaur(q, asker, id) || merfolk(q, asker, id) || pirate(q, asker, id) || vampire(q, asker, id)
}

/// "Craft with a Dinosaur, a Merfolk, a Pirate, and a Vampire": four separate objects (the card's ruling: one changeling is
/// not enough), each chosen as what it is for.
static FOUR: Materials = craft::materials_each(
    &[
        MaterialPart { filter: dinosaur, quality: "Dinosaur" },
        MaterialPart { filter: merfolk, quality: "Merfolk" },
        MaterialPart { filter: pirate, quality: "Pirate" },
        MaterialPart { filter: vampire, quality: "Vampire" },
    ],
    any_of_them,
    "Dinosaur, Merfolk, Pirate or Vampire",
);

/// "Each opponent sacrifices a nonland permanent of their choice. Then you may put an exiled creature card used to craft The
/// Grim Captain onto the battlefield under your control tapped and attacking." (CR 702.167c, 508.4) The card enters
/// attacking what its controller chooses, not necessarily what the Captain attacks, and was never declared as an attacker
/// (the card's rulings).
fn edict_and_return_attacking(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    each_opponent_sacrifices(g, you, item.object, 1, |q, id| !q.chars(id).is(CardType::Land))?;
    let creatures: Vec<ObjectId> =
        craft::exiled_cards_used_to_craft(g, item.object).into_iter().filter(|&c| g.chars(c).is(CardType::Creature)).collect();
    let chosen = choose::cards_from_kind(g, you, item.object, Purpose::PutOntoBattlefield, "creature", creatures.clone(), creatures, 1, true)?;
    let Some(&card) = chosen.first() else { return Ok(()) };
    // The Captain is attacking, so there is something for the card to attack: an opponent at least.
    let Some(defender) = choose_attack_target(g, you, item.object)? else { return Ok(()) };
    g.put_onto_battlefield_attacking(card, you, defender, &[])?;
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(THIS_ATTACKS, edict_and_return_attacking, "Whenever The Grim Captain attacks, each opponent sacrifices a nonland permanent of their choice. Then you may put an exiled creature card used to craft The Grim Captain onto the battlefield under your control tapped and attacking.")],
    ..oracle::THE_GRIM_CAPTAIN
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, mill_two, "{T}: Mill two cards."),
        craft::ability(mana("{4}"), &FOUR, "Craft with a Dinosaur, a Merfolk, a Pirate, and a Vampire {4}"),
    ],
    back: Some(&BACK),
    ..oracle::THRONE_OF_THE_GRIM_CAPTAIN
};
