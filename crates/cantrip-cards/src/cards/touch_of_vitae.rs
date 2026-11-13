use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{0}: Untap this creature. Activate only once." (CR 608.2, 701.26b).
fn granted_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_2 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| is_live(q, o))
            .collect::<Vec<ObjectId>>()
    };
    g.untap_together(&objects_2);
    Ok(())
}

/// Grant haste and the untap ability until end of turn, then schedule the draw (CR 611.2a, 603.7b).
fn draw_card_beginning_next_turns(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = targets::object(g, item, 0);
    let kind_0_0_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste]));
    let kind_0_1_0 = {
        let q: &dyn Query = g;
        EffectKind::AddAbility(
            AbilityRef::delayed(
                {
                    let o = q.state().obj(item.object);
                    o.copied_card().unwrap_or(o.card)
                },
                0,
            ),
        )
    };
    instruction(
        g,
        |g| {
            if let Some(object) = objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_0_1_0, Duration::EndOfTurn);
            }
        },
    );
    g.end_sentence();
    g.delayed_trigger(
        DelayedWhen::StepOfNextTurn {
            step: Step::Upkeep,
            of: None,
        },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 1),
        Target::None,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], draw_card_beginning_next_turns),
    delayed: &[
        ability(
                Cost::mana(ManaCost {
                    generic: 0,
                    x: 0,
                    symbols: [0, 0, 0, 0, 0, 0],
                    hybrid: [None, None, None, None, None, None],
                    twobrid: [0, 0, 0, 0, 0],
                    phyrexian: [0, 0, 0, 0, 0],
                    snow: 0,
                    ..ManaCost::ZERO
                }),
                granted_1,
                "{0}: Untap this creature. Activate only once.",
            )
            .once(),
        delayed_trigger(
            |g, item| effects::draw(g, item, 1),
            "Draw a card at the beginning of the next turn's upkeep.",
        ),
    ],
    ..oracle::TOUCH_OF_VITAE
};
