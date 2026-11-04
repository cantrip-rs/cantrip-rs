use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT;
use cantrip_mechanics::{effects, select, top_of_library};
use cantrip_oracle as oracle;

/// "Exile the top card of that player's library. Then you may sacrifice another creature or artifact. If you do, you may play the
/// exiled card this turn, and mana of any type can be spent to cast it." (CR 406, 608.2c-d, 609.4b, 701.21): the card is exiled
/// first, whatever is done next; sacrificing is the controller's choice as the ability resolves, and nothing is asked when
/// Reno and Rude is the only creature or artifact they have. Only when something was sacrificed is the card playable, until the
/// end of the turn (CR 514.2), with the timing, costs and land-play limits of any other card played (the ruling): a land among
/// them is played as a land. A card that did not reach exile (a replacement sent it elsewhere) is not playable.
fn exile_and_sacrifice_to_play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(damaged) = item.subject else { return Ok(()) };
    let exiled: Vec<ObjectId> = g.exile_all(&top_of_library(g, damaged, 1)).into_iter().filter(|&c| g.obj(c).zone.is(ZoneKind::Exile)).collect();
    let rule = select::another_creature_or_artifact_permanents(select::up_to(1));
    if effects::may_sacrifice(g, item.controller, item.object, rule)?.is_empty() {
        return Ok(());
    }
    for card in exiled {
        g.allow_play(card, item.controller, PlayGrant { any_type: true, lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT,
        exile_and_sacrifice_to_play,
        "Whenever Reno and Rude deals combat damage to a player, exile the top card of that player's library. Then you may sacrifice another creature or artifact. If you do, you may play the exiled card this turn, and mana of any type can be spent to cast it.",
    )],
    ..oracle::RENO_AND_RUDE
};
