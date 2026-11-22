use cantrip_api::*;
use cantrip_mechanics::mana::enters_tapped;
use cantrip_mechanics::{mayhem, triggers};
use cantrip_oracle as oracle;

/// "When this land enters from a graveyard" (CR 603.6a, 400.7): its own entering, as the move says where it came from, a land
/// played from a graveyard by mayhem or put onto the battlefield from one.
fn this_enters_from_a_graveyard(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { from, .. } if from.is(ZoneKind::Graveyard) => triggers::this_enters(q, source, event),
        _ => None,
    }
}

fn you_lose_2_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.lose_life(item.controller, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(when(EventMask::ZONE_CHANGE, this_enters_from_a_graveyard), you_lose_2_life, "When this land enters from a graveyard, you lose 2 life."),
        mana_ability(TAP, add_one_of(&[Color::Blue, Color::Black, Color::Red]), "{T}: Add {U}, {B}, or {R}."),
    ],
    enters: Some(enters_tapped),
    // "Mayhem" without a cost (CR 702.187c): played from the graveyard if it was discarded this turn.
    exile_casts: &[mayhem::PLAY],
    ..oracle::OSCORP_INDUSTRIES
};
