use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Target non-Wall creature an opponent controls" (CR 115.1, 205.3m): a creature on the battlefield an opponent controls,
/// without the creature type Wall as read through this card's words (CR 612.2).
fn non_wall_creature_an_opponent_controls(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::creature_filter(q, asker, t)
        && matches!(t, Target::Object(id) if q.state().opponents(asker.controller).any(|p| p == q.controller(id))
            && !q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Wall)))
}

/// "At the beginning of your upkeep, target non-Wall creature an opponent controls gains forestwalk until your next upkeep." (CR 608.2, 611.2a).
fn target_non_wall_creature_opponent(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = targets::object(g, item, 0);
    let duration_0 = Duration::UntilNextStepOf {
        step: Step::Upkeep,
        player: Some(item.controller),
    };
    let kind_0_0_0 = {
        let q: &dyn Query = g;
        EffectKind::AddKeywords(
            Keyword::landwalk(q.words(item.object).subtype(Subtype::Forest))
                .map_or(Keywords::NONE, |walk| Keywords::of(&[walk])),
        )
    };
    instruction(
        g,
        |g| {
            if let Some(object) = objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, duration_0);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                triggers::YOUR_UPKEEP,
                target_non_wall_creature_opponent,
                "At the beginning of your upkeep, target non-Wall creature an opponent controls gains forestwalk until your next upkeep.",
            )
            .targets(&[target(non_wall_creature_an_opponent_controls)]),
    ],
    ..oracle::ERHNAM_DJINN
};
