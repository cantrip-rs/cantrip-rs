use cantrip_api::*;
use cantrip_mechanics::{attach, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, you may pay {W}{W}. If you do, you gain 1 life." (CR 608.2, 119.3, 118.1).
fn granted_0(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let paid_1 = g
        .may_pay(
            item.controller,
            item.object,
            item.object,
            cost("{W}{W}"),
            &PAY_QUESTION,
        )?;
    g.end_sentence();
    if (paid_1 as i64) != 0 {
        g.gain_life(item.controller, 1);
        g.end_sentence();
    }
    Ok(())
}

/// "Enchanted land has "At the beginning of your upkeep, you may pay {W}{W}. If you do, you gain 1 life."" (CR 604.1).
fn enchanted_land_has_beginning_upkeep_apply_0(
    q: &dyn LayerQuery,
    source: ObjectId,
    chars: &mut Chars,
) {
    let asker = &Asker::new(source, q.controller(source));
    let card = {
        let o = q.state().obj(asker.source);
        o.copied_card().unwrap_or(o.card)
    };
    let _ = card;
    let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) else { return };
    chars.abilities[slot] = Some(
        AbilityRef::delayed(
            {
                let o = q.state().obj(asker.source);
                o.copied_card().unwrap_or(o.card)
            },
            0,
        ),
    );
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}?",
    "",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            attach::attached_object,
            enchanted_land_has_beginning_upkeep_apply_0,
            "Enchanted land has \"At the beginning of your upkeep, you may pay {W}{W}. If you do, you gain 1 life.\"",
        ),
    ],
    delayed: &[
        triggered(
            triggers::YOUR_UPKEEP,
            granted_0,
            "At the beginning of your upkeep, you may pay {W}{W}. If you do, you gain 1 life.",
        ),
    ],
    enchant: Some(cantrip_mechanics::targets::LAND),
    ..oracle::FARMSTEAD
};
