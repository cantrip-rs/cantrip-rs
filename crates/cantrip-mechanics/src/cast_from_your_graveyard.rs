//! "You may cast this card from your graveyard. If you do, it enters with a finality counter on it" (CR 601.3, 614.1c, 122.1):
//! a card's own permission to be cast from its owner's graveyard (`ExileCast` from `ZoneKind::Graveyard`, read off the card
//! there only while it has its abilities, CR 613.1f), and its entry rider, which reads from the history whether the permanent
//! was the spell cast by that permission. Hundred-Battle Veteran and Storied Veteran. A card whose permission costs more
//! (Alien Symbiosis, Wickerfolk Indomitable, Quilled Greatwurm) takes `PERMISSION` with its own `additional` cost.

use cantrip_api::*;

use crate::history;

/// "You may cast this card from your graveyard" (CR 601.2, 404.1): this card, by its owner.
pub fn this_card_by_its_owner(_: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    card == asker.source && player == asker.controller
}

/// The permission, for the card's own mana cost: the card's first `CardDef::exile_casts` entry, which
/// `enters_with_finality_if_cast_by_it` asks after.
pub const PERMISSION: ExileCast = ExileCast {
    text: "Cast from your graveyard",
    from: ZoneKind::Graveyard,
    allows: this_card_by_its_owner,
    additional: Cost::NONE,
    any_type: false,
    any_color: false,
    lands: false,
    life_for_mana_value: false,
    instead: None,
    spell_replacement: None,
};

/// "If you do, it enters with a finality counter on it" (CR 122.1, 614.1c, 601.3): the card's `CardDef::enters`, adding the
/// counter when the spell it was was cast by its own `PERMISSION`. A cast from the graveyard by another permission (Yawgmoth's
/// Will, Lurrus of the Dream-Den) is not "if you do", so it enters without one.
pub fn enters_with_finality_if_cast_by_it(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    enters_with_counter_if_cast_by_it(g, entering, CounterKind::Finality)
}

/// "If you do, it enters with a [`kind`] counter on it" (CR 122.1, 614.1c, 601.3): the rider `enters_with_finality_if_cast_by_it`
/// is, with any one counter. Undead Sprinter's +1/+1 counter is it.
pub fn enters_with_counter_if_cast_by_it(g: &mut dyn Game, entering: &mut Entering, kind: CounterKind) -> Result<(), Suspend> {
    let own = AbilityRef::exile_cast(g.obj(entering.object).card, 0);
    if history::cast_of(g, entering.object).and_then(|cast| cast.granted_by()).is_some_and(|by| by.ability == own) {
        entering.counters[kind as usize] += 1;
    }
    Ok(())
}

/// "You may cast [type] spells from your graveyard" (CR 601.3, 604.1) as the `cost` of a `GrantedWay` that is a permission
/// (`GrantedWay::permission`), so an alternative cost may be paid instead of the mana cost it names: the card, if it is a card of
/// type `TYPE` as it would be cast (`chars`, so a creature that is a land too, Dryad Arbor, is no creature spell: CR 305.9)
/// and its controller's own graveyard only. A card with no mana cost can't be cast for it (CR 118.6). Osteomancer Adept, Tarrian's
/// Journal's Tomb of Aclazotz (creature), Noctis, Prince of Lucis (artifact).
pub fn card_of_type<const TYPE: u8>(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> Option<Cost> {
    if player != asker.controller {
        return None;
    }
    let c = q.chars(card);
    if !CardType::ALL.iter().any(|&t| t as u8 == TYPE && c.is(t)) {
        return None;
    }
    let def = q.card(c.card);
    def.has_cost.then(|| Cost::mana(def.cost))
}
