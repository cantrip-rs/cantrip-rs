use cantrip_api::*;
use cantrip_mechanics::{disguise, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target permanent you own but don't control" (CR 108.3, 109.4, 115.1): a permanent on the battlefield whose owner is the
/// ability's controller and whose controller is somebody else, judged again as the ability resolves (CR 608.2b).
fn permanent_you_own_but_dont_control_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && q.obj(id).owner == asker.controller && q.controller(id) != asker.controller)
}

/// "Gain control of target permanent you own but don't control." (CR 613.1b, 611.2: no duration, so for good).
fn take_back(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    g.gain_control(item.object, permanent, item.controller, Duration::Indefinite);
    Ok(())
}

/// "Target opponent gains control of any number of target permanents you control. Draw a card for each one they gained control of
/// this way." (CR 608.2c): the permanents still legal go to the opponent all at once, and the controller draws for the ones that did
/// change hands, in one draw. An opponent who is no longer a legal target gains nothing, so nothing is drawn.
fn hand_over_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let mut gained = 0;
    instruction(g, |g| {
        for permanent in targets::legal_objects(g, item) {
            if g.gain_control(item.object, permanent, opponent, Duration::Indefinite) {
                gained += 1;
            }
        }
    });
    effects::draw(g, item, gained)
}

pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{1}{U}")), "Disguise {1}{U}")],
    abilities: &[
        triggered(triggers::THIS_ATTACKS, take_back, "Whenever this creature attacks, gain control of target permanent you own but don't control.")
            .targets(&[target(permanent_you_own_but_dont_control_filter)]),
        triggered(
            triggers::THIS_TURNED_FACE_UP,
            hand_over_and_draw,
            "When this creature is turned face up, target opponent gains control of any number of target permanents you control. Draw a card for each one they gained control of this way.",
        )
        .targets(&[targets::OPPONENT, target(targets::permanent_you_control_filter).between(0, TargetSpec::ANY_NUMBER)]),
    ],
    ..oracle::COVETED_FALCON
};
