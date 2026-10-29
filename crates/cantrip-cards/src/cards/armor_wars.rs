use cantrip_api::*;
use cantrip_mechanics::effects::may_draw;
use cantrip_mechanics::{artifacts_you_control, targets};
use cantrip_oracle as oracle;

static DRAW_PER_ARTIFACT: Question =
    Question::new("Draw", "draw a card for each artifact you control?", "If you do, each opponent draws a card", "Draw", "Don't draw");

/// I: "You may draw a card for each artifact you control. If you do, each opponent draws a card." (CR 121.2, 608.2c): the
/// number is counted as the chapter resolves and the choice is all or nothing (one "may" for the whole draw). With no
/// artifact there is no card to draw, so nothing is drawn and "if you do" is false; a player who can't draw that many
/// (CR 121.3) is not offered the choice. Each opponent then draws a card, in turn order.
fn chapter_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = artifacts_you_control(g, item.controller) as u32;
    if count > 0 && may_draw(g, item.controller, item.object, count, &DRAW_PER_ARTIFACT)? {
        let opponents: Vec<PlayerId> = g.state().apnap().filter(|&p| p != item.controller).collect();
        for opponent in opponents {
            g.draw(opponent, 1);
        }
    }
    Ok(())
}

/// II: "Artifact spells you cast this turn cost {1} less to cast." (CR 611.2c, 601.2f): the effect is made for the player
/// and lasts the turn whether or not this Saga stays on the battlefield.
fn chapter_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.spells_cost_this_turn(item, item.controller, 0);
    Ok(())
}

fn artifact_less(q: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::Spell(spell) if q.chars(spell).is(CardType::Artifact) => CostDelta::less(1),
        _ => CostDelta::NONE,
    }
}

/// III: "This Saga deals X damage to target opponent, where X is the greatest mana value among artifacts you control."
/// (CR 608.2h): X is worked out as the chapter resolves, 0 with no artifacts, and a target no longer legal is not dealt
/// anything (CR 608.2b).
fn chapter_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let x = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Artifact)).map(|id| g.mana_value(id)).max().unwrap_or(0);
    g.damage(item.object, Target::Player(opponent), x as u16);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, chapter_one, "I \u{2014} You may draw a card for each artifact you control. If you do, each opponent draws a card."),
        chapter(2, chapter_two, "II \u{2014} Artifact spells you cast this turn cost {1} less to cast."),
        chapter(3, chapter_three, "III \u{2014} This Saga deals X damage to target opponent, where X is the greatest mana value among artifacts you control.")
            .targets(&[targets::OPPONENT]),
    ],
    lasting_cost_changes: &[LastingCostChange { text: "Artifact spells you cast this turn cost {1} less to cast.", change: artifact_less, next: None }],
    ..oracle::ARMOR_WARS
};
