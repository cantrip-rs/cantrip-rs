use cantrip_api::*;
use cantrip_mechanics::{choose, cloak, select, targets};
use cantrip_oracle as oracle;

/// "Target face-down creature" (CR 115.1, 708.2): a face-down permanent on the battlefield is a 2/2 creature, whoever controls it.
fn face_down_creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && q.obj(id).face_down && q.chars(id).is(CardType::Creature))
}

/// "Turn target face-down creature face up" (CR 701.40a, 701.40b, 708.2): the spell's controller turns it up, even where the
/// creature is an opponent's. Nothing happens if the target is no longer a legal target, one already turned up included
/// (CR 608.2b).
fn turn_it_face_up(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    g.turn_face_up(permanent, item.controller);
    Ok(())
}

/// "Exile any number of face-up creatures you control with disguise in a face-down pile, shuffle that pile, then cloak them"
/// (CR 701.58a, 406.3, 110.2a): the creatures are chosen as the mode is carried out from the face-up creatures with disguise the
/// controller then has, exiled together face down, and cloaked together under the controller's control (`cloak::cloak`): the
/// pile's shuffle is the order they are put onto the battlefield in, and they all enter at once, so none is observable. A token
/// is exiled and ceases to exist (CR 111.7): there is no card to cloak.
fn exile_disguise_creatures_face_down_and_cloak_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let eligible: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.controller(id) == you && !g.state().obj(id).face_down && g.chars(id).is(CardType::Creature) && g.chars(id).keywords.has(Keyword::Disguise))
        .collect();
    let rule = Selection::listed(select::ANY_NUMBER, &eligible, eligible.clone()).called("face-up creature with disguise");
    let chosen = choose::select(g, you, item.object, Purpose::Exile, eligible, rule)?;
    let pile = g.exile_face_down(&chosen, None);
    cloak::cloak(g, you, &pile, false)?;
    Ok(())
}

/// "Choose one or both" (CR 700.2): the modes are carried out in the order printed (CR 608.2c), so a creature turned face up by the
/// first can be exiled and cloaked by the second.
static MODAL: ModalDef = modes(&[
    mode(turn_it_face_up, "Turn target face-down creature face up.").targets(&[target(face_down_creature)]),
    mode(
        exile_disguise_creatures_face_down_and_cloak_them,
        "Exile any number of face-up creatures you control with disguise in a face-down pile, shuffle that pile, then cloak them.",
    ),
])
.choose(1, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::EXPOSE_THE_CULPRIT };
