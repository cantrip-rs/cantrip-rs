use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever a player casts an instant spell" (CR 603.2).
fn when_player_casts_instant_spell(
    q: &dyn Query,
    _: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let id = *spell;
    let subject = Target::Object(id);
    let data = [u32::from(player.0), u32::MAX];
    if q.chars(id).is(CardType::Instant) {
        return Some(Fired { subject, data });
    }
    None
}

/// "Whenever a player casts an instant spell, counter it unless that player pays {X}, where X is its mana value." (CR 608.2, 701.6a, 118.1).
fn counter_unless_player_pays_x(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(spell) = item.subject.object() else { return Ok(()) };
    let price = generic(g.mana_value(spell));
    let caster = PlayerId(item.data[0] as u8);
    if !g.may_pay(caster, item.object, item.object, Cost::mana(price), &PAY_QUESTION)? {
        g.counter(spell);
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}? If you don't, this ability attempts to counter the instant.",
    "",
    "Pay {cost}",
    "Don't pay",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, when_player_casts_instant_spell),
            counter_unless_player_pays_x,
            "Whenever a player casts an instant spell, counter it unless that player pays {X}, where X is its mana value.",
        ),
    ],
    ..oracle::IN_THE_EYE_OF_CHAOS
};
