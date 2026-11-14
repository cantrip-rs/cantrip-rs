use cantrip_api::*;
use cantrip_mechanics::{copy, delayed, effects, power_up, targets, triggers};
use cantrip_oracle as oracle;

/// "When you next cast an instant or sorcery spell with mana value less than or equal to Loki's power this turn": a spell
/// cast, not copied (CR 707.10), by the ability's controller (CR 603.7d), whose mana value as it stands on the stack, X
/// included (CR 202.3e), is no more than Loki's power at the moment it is cast, as it last was once Loki has left (CR
/// 608.2h). The spell is the trigger's subject.
fn next_instant_or_sorcery_within_lokis_power(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let fired = triggers::you_cast_instant_or_sorcery(q, me, event)?;
    let spell = fired.subject.object()?;
    (i64::from(q.mana_value(spell)) <= i64::from(targets::power_or_last_known(q, me.source))).then_some(fired)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}").and(TAP),
            delayed::watch_for_the_next,
            "{1}, {T}: When you next cast an instant or sorcery spell with mana value less than or equal to Loki's power this turn, copy that spell. You may choose new targets for the copy.",
        ),
        power_up::ability(
            cost("{4}{R}"),
            effects::two_plus_one_counters_on_this,
            "Power-up \u{2014} {4}{R}: Put two +1/+1 counters on Loki. (Activate each power-up ability only once. Reduce the cost by his mana cost if he entered this turn.)",
        ),
    ],
    delayed: &[triggered(
        when_asked(EventMask::SPELL_CAST, next_instant_or_sorcery_within_lokis_power),
        copy::copy_the_cast_spell,
        "When you next cast an instant or sorcery spell with mana value less than or equal to Loki's power this turn, copy that spell. You may choose new targets for the copy.",
    )],
    ..oracle::LOKI_LAUFEYSON
};
