use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{4}, {T}: For one spell this turn, you may spend mana as though it were mana of any type to pay that spell's mana
/// cost." (CR 609.4b, 611.2a): an effect about the activating player that lasts the turn whether or not this artifact stays,
/// reaching only the mana cost's share of a spell's total cost ("Additional costs are still paid normally", CR 118.8,
/// 601.2f; nor an alternative cost paid instead of the mana cost, CR 118.9) - and for one spell, so the first spell whose
/// payment needed it uses it up. Each activation is one more spell.
fn for_one_spell(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mana_cost_rule_for_one_spell_this_turn(item, 0);
    Ok(())
}

/// "... spend mana as though it were mana of any type to pay that spell's mana cost": mana of any type, {C} included (CR
/// 106.1b), for any spell - the rule is asked only of a spell's mana cost (`CardDef::mana_cost_spend_rules`).
fn any_type(_: &dyn Query, _: &Asker, _: &Spending) -> AnyMana {
    AnyMana::Type
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{4}").and(TAP),
        for_one_spell,
        "{4}, {T}: For one spell this turn, you may spend mana as though it were mana of any type to pay that spell's mana cost. (Additional costs are still paid normally.)",
    )],
    mana_cost_spend_rules: &[ManaCostSpendRule {
        text: "For one spell this turn, you may spend mana as though it were mana of any type to pay that spell's mana cost. (Additional costs are still paid normally.)",
        lets: any_type,
    }],
    ..oracle::NORTH_STAR
};
