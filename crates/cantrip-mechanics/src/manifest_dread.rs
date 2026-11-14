//! Manifest dread (CR 701.62a): "Look at the top two cards of your library. Put one onto the battlefield face down as a
//! 2/2 creature and the other into your graveyard." Abhorrent Oculus does it at the beginning of each opponent's upkeep.

use crate::top_of_library;
use cantrip_api::*;

/// `player` looks at the top two cards of their library, manifests one (CR 701.40a: onto the battlefield face down as a
/// 2/2 creature) and puts the other into their graveyard - both, when the one chosen can't enter (CR 701.40f, 701.62a). A library of one card manifests that one; of none, nothing
/// happens. The player is always asked, as the prompt names which card is which.
pub fn manifest_dread(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<(), Suspend> {
    manifest_dread_object(g, player, source).map(|_| ())
}

/// "Manifest dread" by the controller of the spell or ability (CR 701.62a): the effect of Abhorrent Oculus's upkeep trigger, Turn
/// Inside Out's delayed one and Unsettling Twins's enters trigger.
pub fn controller_manifests_dread(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    manifest_dread(g, item.controller, item.object)
}

/// Manifest dread as `manifest_dread` does, answering the face-down permanent it made ("manifest dread, then attach this
/// Equipment to that creature": Dissection Tools), or `None` when the
/// library was empty.
pub fn manifest_dread_object(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<Option<ObjectId>, Suspend> {
    let looked = top_of_library(g, player, 2);
    if looked.is_empty() {
        return Ok(None);
    }
    let ask = RequestKind::DistributeCards {
        source,
        cards: looked.clone(),
        destinations: vec![
            Destination { kind: DestinationKind::BattlefieldFaceDown, min: 1, max: 1, eligible: looked.clone() },
            Destination { kind: DestinationKind::Graveyard, min: 0, max: u8::MAX, eligible: looked.clone() },
        ],
        may_shuffle: false,
    };
    let mut manifested = looked[0];
    if let Answer::Placements { placements, .. } = g.ask(player, ask)?
        && let Some(&card) = placements.first().and_then(|p| p.first())
    {
        manifested = card;
    }
    let made = crate::manifest::manifest(g, player, manifested)?;
    // "Then put the cards you looked at that were not manifested this way into your graveyard" (CR 701.62a): the chosen one
    // too when it wasn't, a rule or effect keeping it from entering (CR 701.40f) - every looked-at card still where it was.
    let rest: Vec<(ObjectId, Zone)> =
        looked.into_iter().filter(|&c| g.obj(c).became.is_none()).map(|c| (c, Zone::graveyard(player))).collect();
    g.move_together(&rest, MoveReason::Other);
    Ok(made)
}
