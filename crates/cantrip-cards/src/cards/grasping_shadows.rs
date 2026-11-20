use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "It gains deathtouch and lifelink until end of turn. Put a dread counter on this enchantment. Then if there are three or
/// more dread counters on it, transform it." The creature is the trigger's subject, the one that attacked alone (CR 506.5),
/// whichever creature of its controller's it is; both keywords are one instruction (CR 608.2c, 611.2a). The counters stay on
/// it as it transforms, and are counted as the second sentence leaves them (CR 608.2c). The transform is the ability's own
/// instruction about its source, which does nothing if it has transformed since the trigger went on the stack (CR 701.27f).
fn creature_gains_deathtouch_and_lifelink_then_a_dread_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject {
        g.add_effect(item.object, creature, EffectKind::AddKeywords(Keywords::of(&[Keyword::Deathtouch, Keyword::Lifelink])), Duration::EndOfTurn);
    }
    g.end_sentence();
    g.add_counters(item.object, CounterKind::Dread, 1);
    g.end_sentence();
    if g.obj(item.object).counters(CounterKind::Dread) >= 3 {
        g.transform_by(item.object, item)?;
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::B), "{T}: Add {B}."),
        ability(
            cost("{B}, {T}").and(remove_counters(CounterKind::Dread, 1)),
            effects::draw_a_card_and_lose_1_life,
            "{B}, {T}, Remove a dread counter from this land: You draw a card and you lose 1 life.",
        ),
    ],
    ..oracle::SHADOWS_LAIR
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::CREATURE_YOU_CONTROL_ATTACKS_ALONE,
        creature_gains_deathtouch_and_lifelink_then_a_dread_counter,
        "Whenever a creature you control attacks alone, it gains deathtouch and lifelink until end of turn. Put a dread counter on this enchantment. Then if there are three or more dread counters on it, transform it.",
    )],
    back: Some(&BACK),
    ..oracle::GRASPING_SHADOWS
};
