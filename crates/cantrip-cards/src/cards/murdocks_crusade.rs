use cantrip_api::*;
use cantrip_mechanics::{exile_target, targets, teamwork};
use cantrip_oracle as oracle;

/// Either mode: "Exile target ..." (CR 701.13, 608.2b), the target of its own slot (CR 700.2c); one gone or no longer legal by
/// resolution is left alone, the other mode still doing its part.
fn exile_its_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_target(g, item, 0);
    Ok(())
}

/// "Target enchantment with mana value 4 or greater" (CR 115.1, 202.3): an enchantment on the battlefield, its mana value as
/// the layers have it, checked as it is chosen and again as the spell resolves (CR 608.2b).
fn enchantment_with_mana_value_4_or_greater(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_with_mana_value_at_least::<4>(q, asker, t) && matches!(t, Target::Object(id) if q.chars(id).is(CardType::Enchantment))
}

/// "Choose one. If this spell was cast using teamwork, choose both instead." (CR 700.2, 702.194b): with teamwork both modes, each with
/// its own target (CR 700.2c), done in printed order (CR 700.2d).
static MODAL: ModalDef = modes(&[
    mode(exile_its_target, "Street Justice \u{2014} Exile target creature with toughness 4 or greater.")
        .targets(&[targets::CREATURE_WITH_TOUGHNESS_4_OR_GREATER]),
    mode(exile_its_target, "Legal Justice \u{2014} Exile target enchantment with mana value 4 or greater.")
        .targets(&[target(enchantment_with_mana_value_4_or_greater)]),
])
.when_paid(2, 2);

/// Teamwork 4 (as an additional cost to cast this spell, you may tap any number of creatures you control with total power 4 or
/// more): announced before the modes, since how many are chosen is defined by it (CR 601.2b), and offered only while targets for
/// both modes exist (CR 700.2a).
pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    optional_cost: Some(teamwork::optional(4)),
    labels: &["Cast without teamwork (choose one)", "Teamwork 4: Tap any number of creatures you control with total power 4 or more (choose both)"],
    ..oracle::MURDOCKS_CRUSADE
};
