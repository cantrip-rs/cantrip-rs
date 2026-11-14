//! "You may cast this card from your graveyard. If you do, it enters with a finality counter on it" (CR 601.3, 614.1c, 122.1):
//! a card's own permission to be cast from its owner's graveyard (`ExileCast` from `ZoneKind::Graveyard`, read off the card
//! there only while it has its abilities, CR 613.1f), and its entry rider, which reads from the history whether the permanent
//! was the spell cast by that permission. Hundred-Battle Veteran and Storied Veteran. A card whose permission costs more
//! (Alien Symbiosis, Wickerfolk Indomitable, Quilled Greatwurm) takes `PERMISSION` with its own `additional` cost.

use cantrip_api::*;

use crate::history;

/// "You may cast this card from your graveyard" (CR 601.2, 404.1): this card, by its owner.
fn this_card_by_its_owner(_: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
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
};

/// "If you do, it enters with a finality counter on it" (CR 122.1, 614.1c, 601.3): the card's `CardDef::enters`, adding the
/// counter when the spell it was was cast by its own `PERMISSION`. A cast from the graveyard by another permission (Yawgmoth's
/// Will, Lurrus of the Dream-Den) is not "if you do", so it enters without one.
pub fn enters_with_finality_if_cast_by_it(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let own = AbilityRef::exile_cast(g.obj(entering.object).card, 0);
    if history::cast_of(g, entering.object).and_then(|cast| cast.granted_by()).is_some_and(|by| by.ability == own) {
        entering.counters[CounterKind::Finality as usize] += 1;
    }
    Ok(())
}
