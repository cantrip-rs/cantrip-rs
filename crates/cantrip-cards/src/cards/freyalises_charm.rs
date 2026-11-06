use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever an opponent casts a black spell" (CR 603.2).
fn when_opponent_casts_black_spell(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let id = *spell;
    let subject = Target::Object(id);
    if q
        .state()
        .opponents(asker.controller)
        .collect::<Vec<PlayerId>>()
        .contains(&*player)
        && q.chars(id).colors.has(q.words(asker.source).color(Color::Black))
    {
        return Fired::about(subject);
    }
    None
}

/// "Whenever an opponent casts a black spell, you may pay {G}{G}. If you do, you draw a card." (CR 608.2, 121.1, 118.1).
fn pay_g_g_if_do(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let paid_0 = g
        .may_pay(
            item.controller,
            item.object,
            item.object,
            cost("{G}{G}"),
            &PAY_QUESTION,
        )?;
    g.end_sentence();
    if (paid_0 as i64) != 0 {
        g.draw(item.controller, 1);
        g.end_sentence();
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Draw",
    "pay {cost} to draw a card?",
    "",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, when_opponent_casts_black_spell),
            pay_g_g_if_do,
            "Whenever an opponent casts a black spell, you may pay {G}{G}. If you do, you draw a card.",
        ),
        ability(
            cost("{G}{G}"),
            effects::return_this_to_hand,
            "{G}{G}: Return this enchantment to its owner's hand.",
        ),
    ],
    ..oracle::FREYALISES_CHARM
};
