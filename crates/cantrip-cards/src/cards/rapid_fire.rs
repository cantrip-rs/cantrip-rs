use cantrip_api::*;
use cantrip_mechanics::{combat, effects, rampage, targets};
use cantrip_oracle as oracle;

/// "Target creature gains first strike until end of turn. If it doesn't have rampage, that creature gains rampage 2 until end of turn. (Whenever the creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)" (CR 608.2, 611.2a).
fn first_strike_and_rampage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
    g.end_sentence();
    if !g.has_keyword_family(creature, KeywordFamily::Rampage) {
        g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(item.ability.card, 0)), Duration::EndOfTurn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], first_strike_and_rampage),
    delayed: &[rampage::trigger::<2>("Rampage")],
    // "Cast this spell only before blockers are declared." (CR 506.8b, 506.8d)
    cast_only_if: Some(combat::before_blockers_are_declared),
    ..oracle::RAPID_FIRE
};
