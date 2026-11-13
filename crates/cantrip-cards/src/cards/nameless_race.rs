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
    // "The total number of white nontoken permanents your opponents control plus the total number of white cards in their
    // graveyards" (CR 109.2, 109.2a, 111.1), read through this card's words (CR 612): no more than you can pay (CR 119.4).
    let most_0: u32 = {
        let q: &dyn Query = g;
        let white = q.words(asker.source).color(Color::White);
        let theirs = |p: PlayerId| q.state().opponents(entering.controller).any(|o| o == p);
        let permanents = q.state().battlefield().iter()
            .filter(|&&id| q.chars(id).colors.has(white) && !q.state().obj(id).token && theirs(q.controller(id)))
            .count();
        let cards = q.state().apnap()
            .filter(|&p| theirs(p))
            .flat_map(|p| q.state().zone(Zone::graveyard(p)).iter().copied())
            .filter(|&id| !q.state().obj(id).token && !q.state().obj(id).card_copy && q.chars(id).colors.has(white))
            .count();
        count_u32(permanents) + count_u32(cards)
    }
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
    entering.chosen_number = Some(if paid_0 { selected_0 } else { 0 });
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
