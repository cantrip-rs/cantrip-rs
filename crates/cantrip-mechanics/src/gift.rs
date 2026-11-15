//! Gift (CR 702.174): "As an additional cost to cast this spell, you may choose an opponent" (CR 702.174a), and the
//! gift that opponent receives. Dawn's Truce (a card) and Into the Flood Maw (a tapped Fish) print it on instants.
//!
//! The core owns the cost and its timing: the promise and the opponent are announced before the targets (CR 601.2b,
//! 702.174m), and an instant or sorcery gives its gift as it resolves, before its other effects (CR 702.174j), and not at
//! all if it is countered or all its targets are illegal (CR 608.2b). A card with gift gives one of the constants here as
//! its `gift`; "if the gift was promised" is `item.gift.is_some()`, and `promised` reads it for a target filter
//! (`q.obj(asker.source).gift`, the spell on the stack).
//!
//! A permanent with gift lists `trigger(text)` among its `abilities`: "When this permanent enters, if its gift cost was
//! paid, [gift]" (CR 702.174b). A token a gift creates is listed in the card's `EXTRA` (`tokens::FISH`, ...).

use crate::tokens;
use cantrip_api::*;

fn give_card(g: &mut dyn Game, to: PlayerId) -> Result<(), Suspend> {
    g.draw(to, 1);
    Ok(())
}

fn give_food(g: &mut dyn Game, to: PlayerId) -> Result<(), Suspend> {
    tokens::create_food(g, to)
}

fn give_treasure(g: &mut dyn Game, to: PlayerId) -> Result<(), Suspend> {
    tokens::create_treasure(g, to)
}

fn give_tapped_fish(g: &mut dyn Game, to: PlayerId) -> Result<(), Suspend> {
    tokens::create_by_rule(g, to, &tokens::FISH, 1, true)?;
    Ok(())
}

fn give_octopus(g: &mut dyn Game, to: PlayerId) -> Result<(), Suspend> {
    tokens::create_by_rule(g, to, &tokens::OCTOPUS, 1, false)?;
    Ok(())
}

fn give_extra_turn(g: &mut dyn Game, to: PlayerId) -> Result<(), Suspend> {
    g.add_extra_turns(to, 1);
    Ok(())
}

/// "Gift a Food" (CR 702.174d): the chosen player creates a Food token. List `tokens::FOOD` in the card's `EXTRA`.
pub const FOOD: GiftDef = GiftDef { kind: Gift::Food, give: give_food };
/// "Gift a card" (CR 702.174e): the chosen player draws a card.
pub const CARD: GiftDef = GiftDef { kind: Gift::Card, give: give_card };
/// "Gift a tapped Fish" (CR 702.174f): the chosen player creates a tapped 1/1 blue Fish creature token. List `tokens::FISH`
/// in the card's `EXTRA`.
pub const TAPPED_FISH: GiftDef = GiftDef { kind: Gift::TappedFish, give: give_tapped_fish };
/// "Gift an extra turn" (CR 702.174g): the chosen player takes an extra turn after this one.
pub const EXTRA_TURN: GiftDef = GiftDef { kind: Gift::ExtraTurn, give: give_extra_turn };
/// "Gift a Treasure" (CR 702.174h): the chosen player creates a Treasure token. List `tokens::TREASURE` in the card's `EXTRA`.
pub const TREASURE: GiftDef = GiftDef { kind: Gift::Treasure, give: give_treasure };
/// "Gift an Octopus" (CR 702.174i): the chosen player creates an 8/8 blue Octopus creature token. List `tokens::OCTOPUS`
/// in the card's `EXTRA`.
pub const OCTOPUS: GiftDef = GiftDef { kind: Gift::Octopus, give: give_octopus };

/// "If the gift was promised": the spell on the stack was cast with the opponent chosen (CR 702.174k), for a target
/// filter that depends on it (CR 702.174m).
pub fn promised(q: &dyn Query, spell: ObjectId) -> bool {
    q.obj(spell).gift.is_some() || q.gift_assumed()
}

/// A target that depends on the promise: "target [A]. If the gift was promised, instead target [B]" (CR 702.174m): the spell on
/// the stack is judged against `if_promised` when the gift was promised and against `otherwise` when it was not, both when the
/// target is announced (CR 601.2b-c) and again as the spell resolves (CR 608.2b). Into the Flood Maw and Long River's Pull use it.
pub fn target_by_promise(q: &dyn Query, asker: &Asker, t: Target, if_promised: &TargetSpec, otherwise: &TargetSpec) -> bool {
    let filter = if promised(q, asker.source) { if_promised } else { otherwise };
    (filter.filter)(q, asker, t)
}

/// The intervening "if" (CR 603.4, 702.174b): its gift cost was paid. The gift triggered ability has it, and so does a
/// permanent's own "when this enters, if the gift was promised" (Scrapshooter).
pub fn paid(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).gift.is_some()
}

/// The gift triggered ability's effect: the opponent the permanent was cast promising it to is given its gift as the
/// ability resolves (CR 702.174b), unless they have left the game (CR 800.4a). The opponent is the one the ability
/// carries from when it triggered (`item.gift`), so the gift is still given if the permanent has left the battlefield
/// (CR 603.10).
fn give_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let (Some(to), Some(gift)) = (item.gift, g.card(g.obj(item.object).card).gift) else { return Ok(()) };
    if !g.state().player(to).lost {
        (gift.give)(g, to)?;
    }
    g.gave_gift(item.controller, to, gift.kind);
    Ok(())
}

/// "When this permanent enters, if its gift cost was paid, [gift]" (CR 702.174b).
pub const fn trigger(text: &'static str) -> AbilityDef {
    triggered(crate::triggers::THIS_ENTERS, give_it, text).condition(paid)
}
