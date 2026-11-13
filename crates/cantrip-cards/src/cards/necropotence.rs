use cantrip_api::*;
use cantrip_mechanics::{top_of_library, triggers::YOU_DISCARD};
use cantrip_oracle as oracle;

/// "Skip your draw step." (CR 614.10).
fn skip_draw(_: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    player == asker.controller
}

/// The discarded object must still be in your graveyard (CR 400.7, 701.13a).
fn exile_card_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(card) = item.subject
        && is_live(g, card)
        && g.obj(card).zone == Zone::graveyard(item.controller)
    {
        g.exile_all(&[card]);
    }
    Ok(())
}

/// "Put that card into your hand at the beginning of your next end step." (CR 608.2, 400.7).
fn delayed_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_4 = {
        let q: &dyn Query = g;
        delayed_3_capture_0(q, item)
            .first()
            .copied()
            .into_iter()
            .filter(|&id| is_live(q, id) && q.obj(id).zone.is(ZoneKind::Exile))
            .collect::<Vec<ObjectId>>()
    };
    g.return_to_hand(&objects_4);
    Ok(())
}

/// "Put that card into your hand at the beginning of your next end step." (CR 608.2).
fn delayed_3_capture_0(q: &dyn Query, item: &StackItem) -> Vec<ObjectId> {
    about(q, item, 0)
}

/// "Pay 1 life: Exile the top card of your library face down. Put that card into your hand at the beginning of your next end step." (CR 608.2, 701.13a).
fn exile_top_card_library_face(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_1 = top_of_library(g, item.controller, 1);
    let moved_1 = g.exile_face_down(&objects_1, None);
    g.end_sentence();
    let about: Vec<ObjectId> = moved_1
        .clone()
        .get(0)
        .copied()
        .into_iter()
        .collect::<Vec<ObjectId>>();
    g.delayed_trigger_about(
        DelayedWhen::Step {
            step: Step::End,
            on_turn_of: Some(item.controller),
        },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        &about,
    );
    Ok(())
}

/// "Whenever you discard a card, exile that card from your graveyard." (CR 608.2).
fn about(q: &dyn Query, item: &StackItem, part: usize) -> Vec<ObjectId> {
    assert_eq!(part, 0, "one real-object list");
    q.event_objects(item).to_vec()
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            YOU_DISCARD,
            exile_card_graveyard,
            "Whenever you discard a card, exile that card from your graveyard.",
        ),
        ability(
            life(1),
            exile_top_card_library_face,
            "Pay 1 life: Exile the top card of your library face down. Put that card into your hand at the beginning of your next end step.",
        ),
    ],
    delayed: &[
        delayed_trigger(
            delayed_3,
            "Put that card into your hand at the beginning of your next end step.",
        ),
    ],
    skip_steps: &[
        SkipStep {
            applies: skip_draw,
            step: Step::Draw,
        },
    ],
    ..oracle::NECROPOTENCE
};
