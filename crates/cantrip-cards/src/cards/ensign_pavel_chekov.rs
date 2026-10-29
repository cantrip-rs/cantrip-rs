use cantrip_api::*;
use cantrip_mechanics::{choose, lands, select};
use cantrip_oracle as oracle;

/// The card chosen for an opponent must be a land and owned by that opponent (CR 608.2d).
fn owned_by(q: &dyn Query, choice: &cantrip_api::select::Choice, owner: u32) -> cantrip_api::select::Verdict {
    cantrip_api::select::Verdict::each(choice, |card| q.obj(card).owner.0 as u32 == owner)
}

fn land_owned_by(owner: PlayerId) -> Selection {
    Selection::new(select::exactly(1))
        .with(select::LAND)
        .with(select::part(&(owned_by as cantrip_api::select::SelectFn), owner.0 as u32, "owned by that opponent"))
        .called("land card")
}

/// "When Ensign Chekov enters, each opponent exiles the top X cards of their library, where X is the number of differently
/// named lands you control. For each opponent, choose a land card that player owns exiled this way. You may play the chosen
/// cards for as long as they remain exiled." The count is the distinct land names of the ability's controller at resolution
/// (CR 201.2b, 608.2c). Every opponent's top cards are exiled together (CR 101.4, 406.2); a replacement that sends a card
/// elsewhere leaves it out of the later choices. The controller chooses one of that opponent's owned land cards among the
/// cards exiled this way (CR 608.2d), then receives permission to play it as a land from exile for as long as it stays
/// there (CR 305.1, 400.7, 601.3, 701.18).
fn strange_new_worlds(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let count = lands::differently_named_lands(g, you);
    let opponents: Vec<PlayerId> = g.state().opponents(you).collect();
    let top: Vec<ObjectId> = opponents.iter().flat_map(|&opponent| cantrip_mechanics::top_of_library(g, opponent, count)).collect();
    let exiled: Vec<ObjectId> = g.exile_all(&top).into_iter().filter(|&card| g.obj(card).zone.is(ZoneKind::Exile)).collect();

    for opponent in opponents {
        let rule = land_owned_by(opponent);
        let choice = cantrip_api::select::Choice { chooser: you, source: item.object, candidates: &exiled, selected: &[] };
        if !cantrip_api::select::judge(g, &rule, &choice).addable.any() {
            continue;
        }
        let chosen = choose::select(g, you, item.object, Purpose::Play, exiled.clone(), rule)?;
        for card in chosen {
            g.allow_play(card, you, PlayGrant { lands: true, expiry: Expiry::Never, ..PlayGrant::CAST });
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(cantrip_mechanics::triggers::THIS_ENTERS, strange_new_worlds, "Strange new worlds — When Ensign Chekov enters, each opponent exiles the top X cards of their library, where X is the number of differently named lands you control. For each opponent, choose a land card that player owns exiled this way. You may play the chosen cards for as long as they remain exiled.")],
    ..oracle::ENSIGN_PAVEL_CHEKOV
};
