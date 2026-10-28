use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Counter target spell. If a permanent spell is countered this way, exile it instead of putting it into its owner's
/// graveyard. You may cast that card without paying its mana cost for as long as it remains exiled." (CR 701.6, 110.4b, 118.9,
/// 400.7): a permanent spell is judged as it is on the stack; one that can't be countered stays there and so is not exiled or
/// cast; the permission is to the card in exile and lasts for as long as it stays there (a card that leaves is a new object),
/// timing restrictions and permissions still applying (the ruling), X 0 for a cost with {X} (CR 107.3b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    if !targets::is_permanent_card(&g.chars(spell)) {
        g.counter(spell);
        return Ok(());
    }
    g.counter_to(spell, CounterTo::Exile);
    if let Some(card) = g.found_after_move(spell)
        && g.obj(card).zone.is(ZoneKind::Exile)
    {
        g.allow_play(card, item.controller, PlayGrant { instead: Some(ManaCost::ZERO), expiry: Expiry::Never, ..PlayGrant::CAST });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::SPELL], effect), ..oracle::THRANDUILS_DECREE };
