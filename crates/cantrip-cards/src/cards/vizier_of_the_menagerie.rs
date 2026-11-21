use cantrip_api::*;
use cantrip_mechanics::targets::filters::creature;
use cantrip_oracle as oracle;

/// "You can spend mana of any type to cast creature spells" (CR 609.4b, 106.6): its controller's own casting of a creature spell, from
/// any zone, not an ability's activation, nor a spell of another type. Mana of any type includes {C}, so it pays a colored symbol with
/// colorless mana too (the ruling), and every other cost of the spell, an additional or alternative one, is paid as ever.
fn creature_spells(q: &dyn Query, asker: &Asker, spending: &Spending) -> AnyMana {
    if spending.player == asker.controller
        && spending.purpose == SpendingFor::Casting
        && spending.source.is_some_and(|spell| q.chars(spell).is(CardType::Creature))
    {
        AnyMana::Type
    } else {
        AnyMana::No
    }
}

/// "You may look at the top card of your library any time. You may cast creature spells from the top of your library" (CR 401.5, 601.3).
pub static CARD: CardDef = CardDef {
    play_rules: &[PlayRule::LookAtLibraryTop, PlayRule::LibraryTopSpells(creature, None)],
    spend_any_mana: &[creature_spells],
    ..oracle::VIZIER_OF_THE_MENAGERIE
};
