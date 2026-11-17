use cantrip_api::*;
use cantrip_mechanics::{disguise, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "Exile another target nonland permanent. If you controlled it, return it to the battlefield tapped. Otherwise, its controller
/// creates a 2/2 white and blue Detective creature token." (CR 701.13a, 110.2a, 608.2c, 111.1): "if you controlled it" is judged as
/// the ability resolves, before the exile (the 2024-02-02 ruling: a permanent you controlled earlier in the turn but not now gives
/// its present controller the Detective). The card comes back as a new object (CR 400.7) tapped and, since the ability tells you to
/// put it onto the battlefield, under your control (CR 110.2a) whoever owns it; a token exiled this way is gone for good
/// (CR 111.8) and cannot return. A merged permanent exiled is every card it was made of, and "return it" returns them all,
/// together (CR 730.3c). Nothing happens if the target is no longer legal (CR 608.2b).
fn exile_then_return_or_detective(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    let controller = g.controller(permanent);
    let exiled = g.exile_all(&[permanent]);
    if controller == item.controller {
        let returning: Vec<(ObjectId, PlayerId, bool)> = exiled
            .into_iter()
            .filter(|&card| !g.obj(card).token && g.obj(card).zone.is(ZoneKind::Exile))
            .map(|card| (card, item.controller, true))
            .collect();
        g.put_all_onto_battlefield_as(&returning)?;
    } else {
        tokens::create(g, controller, &tokens::WHITE_BLUE_DETECTIVE, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{1}{W}")), "Disguise {1}{W}")],
    abilities: &[triggered(
        triggers::THIS_TURNED_FACE_UP,
        exile_then_return_or_detective,
        "When this creature is turned face up, exile another target nonland permanent. If you controlled it, return it to the battlefield tapped. Otherwise, its controller creates a 2/2 white and blue Detective creature token.",
    )
    .targets(&[target(targets::other_nonland_permanent_filter)])],
    ..oracle::UNYIELDING_GATEKEEPER
};
pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_BLUE_DETECTIVE];
