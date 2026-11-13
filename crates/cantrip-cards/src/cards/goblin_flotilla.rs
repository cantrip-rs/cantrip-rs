use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of each combat, unless you pay {R}, whenever this creature blocks or becomes blocked by a creature this combat, that creature gains first strike until end of turn." (CR 608.2, 611.2a).
fn watch_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(other) = item.subject.object() { instruction(g, |g| effects::gain_until_end_of_turn(g, item.object, other, Keyword::FirstStrike)); }
    Ok(())
}

/// "At the beginning of each combat, unless you pay {R}, whenever this creature blocks or becomes blocked by a creature this combat, that creature gains first strike until end of turn." (CR 608.2, 118.1).
fn unless_pay_r_whenever_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let paid_0 = g
        .may_pay(item.controller, item.object, item.object, cost("{R}"), &PAY_QUESTION)?;
    g.end_sentence();
    if !(((paid_0 as i64) != 0)) {
        g.delayed_trigger(
            DelayedWhen::ThisCombat,
            item.object,
            item.controller,
            AbilityRef::delayed(item.ability.card, 0),
            Target::None,
        );
        g.end_sentence();
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}?",
    "",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::EACH_PLAYERS_BEGINNING_OF_COMBAT,
            unless_pay_r_whenever_creature,
            "At the beginning of each combat, unless you pay {R}, whenever this creature blocks or becomes blocked by a creature this combat, that creature gains first strike until end of turn.",
        ),
    ],
    delayed: &[
        triggered(
            when_asked(EventMask::BLOCKS, triggers::blocks_or_becomes_blocked),
            watch_2,
            "At the beginning of each combat, unless you pay {R}, whenever this creature blocks or becomes blocked by a creature this combat, that creature gains first strike until end of turn.",
        ),
    ],
    ..oracle::GOBLIN_FLOTILLA
};
