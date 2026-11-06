use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever you're dealt damage" (CR 603.2).
fn when_youre_dealt_damage(
    _: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Damage { target, amount, .. } = event else { return None };
    let subject = *target;
    let data = [*amount, 0];
    if matches!(* target, Target::Player(p) if asker.controller == p) {
        return Some(Fired { subject, data });
    }
    None
}

/// "Whenever you're dealt damage, put that many vitality counters on this Aura." (CR 608.2, 122.1).
fn put_many_vitality_counters_aura(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Vitality, item.matched_damage_amount() as i32);
    Ok(())
}

/// "At the beginning of your upkeep, you may remove a vitality counter from this Aura. If you do, you gain 1 life." (CR 118.12, 119.3).
fn remove_vitality_counter_aura_if(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !targets::permanent(g, item.object) || g.obj(item.object).counters(CounterKind::Vitality) == 0 {
        return Ok(());
    }
    let paid = g.may_pay(item.controller, item.object, item.object, remove_counters(CounterKind::Vitality, 1), &PAY)?;
    g.end_sentence();
    if paid {
        g.gain_life(item.controller, 1);
    }
    Ok(())
}

static PAY: Question = Question::new("RemoveACounter", "remove a vitality counter from this Aura?", "If you do, you gain 1 life", "Remove a counter", "Keep it");

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::DAMAGE, when_youre_dealt_damage),
                put_many_vitality_counters_aura,
                "Whenever you're dealt damage, put that many vitality counters on this Aura.",
            )
            .damage_amount()
            .sums_per_subject(),
        triggered(
            triggers::YOUR_UPKEEP,
            remove_vitality_counter_aura_if,
            "At the beginning of your upkeep, you may remove a vitality counter from this Aura. If you do, you gain 1 life.",
        ),
    ],
    enchant: Some(targets::ARTIFACT),
    ..oracle::LIVING_ARTIFACT
};
