use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever a player casts a white spell" (CR 603.2).
fn when_player_casts_white_spell(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::SpellCast { spell, .. } = event else { return None };
    let id = *spell;
    let subject = Target::Object(id);
    if q.chars(id).colors.has(q.words(asker.source).color(Color::White)) {
        return Fired::about(subject);
    }
    None
}

/// "Whenever a player casts a white spell, you may pay {1}. If you do, you gain 1 life." (CR 608.2, 119.3, 118.1).
fn pay_1_if_do_gain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let paid_0 = g
        .may_pay(item.controller, item.object, item.object, cost("{1}"), &PAY_QUESTION)?;
    g.end_sentence();
    if (paid_0 as i64) != 0 {
        g.gain_life(item.controller, 1);
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
            when_asked(EventMask::SPELL_CAST, when_player_casts_white_spell),
            pay_1_if_do_gain,
            "Whenever a player casts a white spell, you may pay {1}. If you do, you gain 1 life.",
        ),
    ],
    ..oracle::IVORY_CUP
};
