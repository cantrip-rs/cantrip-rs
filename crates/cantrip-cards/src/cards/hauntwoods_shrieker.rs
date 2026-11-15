use cantrip_api::*;
use cantrip_mechanics::{effects::ask_may, manifest_dread::controller_manifests_dread, targets, triggers};
use cantrip_oracle as oracle;

/// "Target face-down permanent" (CR 115.1, 708.2): a permanent on the battlefield that is face down, whoever controls it.
fn face_down_permanent(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && q.obj(id).face_down)
}

const TURN_UP: Question = Question::new("TurnFaceUp", "turn it face up?", "", "Turn it face up", "Leave it face down");

/// "Reveal target face-down permanent. If it's a creature card, you may turn it face up." (CR 701.20a, 701.40a, 708.2, 608.2b): the
/// ability's controller reveals it, so every player sees what it is, and only if it is a creature card (its printed card, not the
/// 2/2 it shows face down) is asked whether to turn it up; a manifested instant or sorcery card is revealed and stays face down
/// (CR 701.40g, 701.58g, in `turn_face_up`). The controller of the ability turns it up (CR 701.40b), even where the permanent is
/// an opponent's. Nothing happens if the target is no longer a legal target, a permanent already turned up included (CR 608.2b).
fn reveal_and_maybe_turn_up(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    g.reveal(item.controller, &[permanent]);
    let card = g.obj(permanent).card;
    if g.card(card).types.has(CardType::Creature) && ask_may(g, item.controller, item.object, Purpose::Ask(&TURN_UP))? {
        g.turn_face_up(permanent, item.controller);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ATTACKS, controller_manifests_dread, "Whenever this creature attacks, manifest dread."),
        ability(cost("{1}{G}"), reveal_and_maybe_turn_up, "{1}{G}: Reveal target face-down permanent. If it's a creature card, you may turn it face up.")
            .targets(&[target(face_down_permanent)]),
    ],
    ..oracle::HAUNTWOODS_SHRIEKER
};
