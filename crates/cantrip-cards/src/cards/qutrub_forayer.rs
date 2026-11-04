use cantrip_api::*;
use cantrip_mechanics::{destroy_target, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Target creature that was dealt damage this turn" (CR 115.1, 120.2): a creature on the battlefield that something dealt
/// damage to this turn, as it is chosen and again as the ability resolves. Damage that was prevented
/// was not dealt (CR 615.6); the object is the one dealt it, so a creature that left and came back has been dealt none (CR 400.7).
fn creature_dealt_damage_this_turn(q: &dyn Query, _: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    q.is_on_battlefield(id) && q.obj(id).became.is_none()
        && q.chars(id).is(CardType::Creature)
        && q.history().damage_dealt().any(|d| d.target == t)
}

/// Mode 1: "Destroy target creature that was dealt damage this turn." (CR 701.8): nothing if the target is no longer legal (CR 608.2b).
fn destroy_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

/// Mode 2: "Exile up to two target cards from a single graveyard." (CR 701.13): the targets still in their graveyard are exiled
/// together; one that became illegal is left alone (CR 608.2b).
fn exile_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.exile_all(&targets::legal_objects(g, item));
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(destroy_it, "Destroy target creature that was dealt damage this turn.").targets(&[target(creature_dealt_damage_this_turn)]),
    mode(exile_them, "Exile up to two target cards from a single graveyard.").targets(targets::UP_TO_TWO_CARDS_FROM_A_SINGLE_GRAVEYARD),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(THIS_ENTERS, &MODAL, "When this creature enters, choose one \u{2014}\n\u{2022} Destroy target creature that was dealt damage this turn.\n\u{2022} Exile up to two target cards from a single graveyard.")],
    ..oracle::QUTRUB_FORAYER
};
