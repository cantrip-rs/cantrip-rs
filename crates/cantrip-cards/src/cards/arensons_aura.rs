use cantrip_api::*;
use cantrip_mechanics::{counter_target, effects, targets};
use cantrip_oracle as oracle;

/// An enchantment spell on the stack (CR 115.1).
fn enchantment_spell(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::SPELL.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if q.chars(id).is(CardType::Enchantment))
}

/// "Counter target enchantment spell." (CR 701.6a).
fn counter_enchantment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counter_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{W}").and(sacrifice(1, targets::filters::enchantment)), effects::destroy_first_target,
            "{W}, Sacrifice an enchantment: Destroy target enchantment.")
            .targets(&[targets::ENCHANTMENT]),
        ability(cost("{3}{U}{U}"), counter_enchantment, "{3}{U}{U}: Counter target enchantment spell.")
            .targets(&[target(enchantment_spell)]),
    ],
    ..oracle::ARENSONS_AURA
};
