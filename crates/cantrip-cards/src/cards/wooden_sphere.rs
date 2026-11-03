use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever a player casts a green spell, you may pay {1}. If you do, you gain 1 life." (CR 608.2, 119.3, 118.1).
fn pay_1_if_do_gain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let paid_0 = g
        .may_pay(item.controller, item.object, item.object, cost("{1}"), &PAY_QUESTION)?;
    g.end_sentence();
    if paid_0 {
        g.gain_life(item.controller, 1);
        g.end_sentence();
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost} to gain 1 life?",
    "",
    "Pay {cost}",
    "Don't pay",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, triggers::a_player_casts_colored_spell::<{Color::Green as u8}>),
            pay_1_if_do_gain,
            "Whenever a player casts a green spell, you may pay {1}. If you do, you gain 1 life.",
        ),
    ],
    ..oracle::WOODEN_SPHERE
};
