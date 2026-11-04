use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, you may untap target tapped creature an opponent controls. If you do, untap this creature." (CR 608.2, 118.12).
fn untap_target_tapped_creature_opponent(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    if g.can_untap(creature)
        && effects::ask_may(g, item.controller, item.object, Purpose::Ask(&UNTAP_QUESTION))?
    {
        g.untap(creature);
        g.end_sentence();
        g.untap(item.object);
    }

    Ok(())
}

static UNTAP_QUESTION: Question = Question::new(
    "UntapCreature",
    "untap the targeted creature to untap this creature?",
    "",
    "Untap it",
    "Leave it tapped",
);

pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::DoesNotUntap]),
    abilities: &[
        triggered(
                triggers::YOUR_UPKEEP,
                untap_target_tapped_creature_opponent,
                "At the beginning of your upkeep, you may untap target tapped creature an opponent controls. If you do, untap this creature.",
            )
            .targets(&[targets::TAPPED_CREATURE_AN_OPPONENT_CONTROLS]),
    ],
    ..oracle::SOLDEVI_GOLEM
};
