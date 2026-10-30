use cantrip_api::*;
use cantrip_mechanics::{choose, creature_types_among_non_borg, targets};
use cantrip_oracle as oracle;

/// "Exile target instant or sorcery card from your graveyard. Copy that card X times, where X is the number of creature
/// types among non-Borg creatures you control. You may cast the copies without paying their mana costs. Exile Infinite
/// Diversity." The target is exiled (a new object, CR 400.7) and the copies are made of that card in exile (CR 707.12), X
/// counted as this resolves (CR 608.2h); each copy is cast while this resolves, whatever its timing (CR 608.2g), and any
/// not cast ceases to exist (CR 707.10a, 704.5e). With the target gone, nothing is exiled or copied (CR 608.2b), but this
/// spell is still exiled.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(card) = targets::object(g, item, 0) {
        let exiled = g.exile(card);
        let x = creature_types_among_non_borg(g, item.object, item.controller);
        let copies: Vec<ObjectId> = (0..x).map(|_| g.create_copy_in_exile(exiled, item.controller)).collect();
        choose::cast_any_number_free(g, item.controller, item.object, &copies)?;
    }
    g.end_sentence();
    g.exile(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD], effect),
    ..oracle::INFINITE_DIVERSITY
};
