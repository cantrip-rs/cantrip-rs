use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// "As this creature enters, pay any amount of life. The amount you pay can't be more than the total number of white nontoken permanents your opponents control plus the total number of white cards in their graveyards. Nameless Race's power and toughness are each equal to the life paid as it entered." (CR 614.1c, 118.1).
fn as_creature_enters_pay_any_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    let asker = &Asker::new(entering.object, entering.controller);
    let payer_0 = entering.controller;
    let most_0: u32 = ({
        let q: &dyn Query = g;
        (q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).colors.has(q.words(asker.source).color(Color::White))
                    && !(q.state().obj(id).token)
                    && q
                        .state()
                        .opponents(entering.controller)
                        .collect::<Vec<PlayerId>>()
                        .contains(&q.controller(id))
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64
            + q
                .state()
                .apnap()
                .collect::<Vec<PlayerId>>()
                .into_iter()
                .flat_map(|p: PlayerId| {
                    q.state().zone(Zone::graveyard(p)).iter().rev().copied()
                })
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .filter(|&id| {
                    !q.state().obj(id).token && !q.state().obj(id).card_copy
                        && (q
                            .chars(id)
                            .colors
                            .has(q.words(asker.source).color(Color::White))
                            && (q.state().obj(id).zone.kind() == ZoneKind::Graveyard
                                && q
                                    .state()
                                    .opponents(entering.controller)
                                    .collect::<Vec<PlayerId>>()
                                    .contains(&q.state().obj(id).owner)))
                })
                .collect::<Vec<ObjectId>>()
                .len() as i64)
            .min(u32::MAX.into()) as u32
    })
        .min(
            if g.can_lose_life(payer_0) {
                g.state().player(payer_0).life.max(0) as u32
            } else {
                0
            },
        );
    let selected_0 = choose::number(g, payer_0, entering.object, 0, most_0, "Pay life")?;
    let paid_0 = selected_0 == 0
        || g
            .may_pay(
                payer_0,
                entering.object,
                entering.object,
                Cost {
                    life: selected_0,
                    ..Cost::NONE
                },
                &PAY_QUESTION,
            )?;
    let bound_0: i64 = if paid_0 { i64::from(selected_0) } else { 0 };
    entering.chosen_number = Some(bound_0 as u32);
    Ok(())
}

/// "Nameless Race's power and toughness are each equal to the life paid as it entered." (CR 604.1).
fn nameless_races_power_toughness_equal_apply_0(
    q: &dyn LayerQuery,
    source: ObjectId,
    chars: &mut Chars,
) {
    // The entry choice is bounded by the payer's i32 life total.
    let paid = q.state().chosen_number(q.linked_to(source)).unwrap_or(0) as i32;
    chars.power = Some(paid);
    chars.toughness = Some(paid);
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
        characteristic_defining(
            Layer::PtDefine,
            nameless_races_power_toughness_equal_apply_0,
            "Nameless Race's power and toughness are each equal to the life paid as it entered.",
        ).lost_pt(0, 0),
    ],
    enters: Some(as_creature_enters_pay_any_enters),
    ..oracle::NAMELESS_RACE
};
