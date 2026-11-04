use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, targets};
use cantrip_oracle as oracle;

/// "Whenever you cast a noncreature spell, if at least four mana was spent to cast it" (CR 603.2, 603.4, 601.2h): the payment is
/// over before the trigger and nothing changes it afterwards, so the condition asked as it triggers is the one asked as it
/// resolves. A spell cast without spending mana has had none spent.
fn noncreature_spell_with_four_mana_spent(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. }
            if *player == me.controller && !q.chars(*spell).is(CardType::Creature) && q.obj(*spell).spent.total() >= 4 =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Blow Up - {T}, Sacrifice this creature: It deals damage equal to its power to target creature." The sacrifice is a cost (CR
/// 601.2h), so it is gone when this resolves and deals damage equal to its power as it last existed on the battlefield (CR 113.7a,
/// 608.2h), counters included; it is the source. A target no longer legal is dealt nothing (CR 608.2b).
fn blow_up(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = g.chars(item.object).power.unwrap_or_default().max(0) as u16;
    damage_target(g, item, 0, power);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, noncreature_spell_with_four_mana_spent),
            effects::plus_one_counter_on_this,
            "Whenever you cast a noncreature spell, if at least four mana was spent to cast it, put a +1/+1 counter on this creature.",
        ),
        ability(
            TAP.and(SACRIFICE_THIS),
            blow_up,
            "Blow Up \u{2014} {T}, Sacrifice this creature: It deals damage equal to its power to target creature. Activate only as a sorcery.",
        )
        .targets(&[targets::CREATURE])
        .sorcery_speed(),
    ],
    ..oracle::BLAZING_BOMB
};
