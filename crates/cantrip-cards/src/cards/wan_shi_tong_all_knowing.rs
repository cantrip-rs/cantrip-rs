use cantrip_api::*;
use cantrip_mechanics::{library, targets, tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Target nonland permanent's owner puts it into their library second from the top or on the bottom: their choice.
fn put_into_library(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    library::owner_puts_second_from_top_or_bottom(g, item.object, permanent)
}

/// "Whenever one or more cards are put into a library from anywhere" (CR 603.2c): cards, not tokens, and not moved within a
/// library. Judged as the permanent was (CR 603.10a). One trigger for each library, which is the batch's `data[1]`.
fn cards_into_a_library(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, new, from, to, .. } if to.is(ZoneKind::Library) && !from.is(ZoneKind::Library) && !q.obj(*old).token => {
            Some(Fired { subject: Target::Object(*new), data: [0, to.player.0 as u32] })
        }
        _ => None,
    }
}

fn make_spirits(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::SPIRIT, 2)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    labels: &["Second from the top", "On the bottom"],
    abilities: &[
        triggered(THIS_ENTERS, put_into_library, "When Wan Shi Tong enters, target nonland permanent's owner puts it into their library second from the top or on the bottom.")
            .targets(&[targets::NONLAND_PERMANENT]),
        triggered(when(EventMask::ZONE_CHANGE, cards_into_a_library), make_spirits, "Whenever one or more cards are put into a library from anywhere, create two 1/1 colorless Spirit creature tokens with \"This token can't block or be blocked by non-Spirit creatures.\"")
            .once_per_batch()
            .batch_by_data(),
    ],
    ..oracle::WAN_SHI_TONG_ALL_KNOWING
};

pub static EXTRA: &[&CardDef] = &[&tokens::SPIRIT];
