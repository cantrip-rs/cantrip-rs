use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target artifact, instant, or sorcery card from your graveyard" (CR 115.1, 400.7): a card in the graveyard of the
/// ability's controller that is any of the three card types.
fn artifact_instant_or_sorcery_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Artifact, CardType::Instant, CardType::Sorcery])
}

/// III: "The owners of those cards shuffle them into their libraries" (CR 701.24a, 101.4): both cards move together,
/// each into its owner's library, and each owner's library is shuffled even where a card has left in the meantime
/// (CR 701.24c, `LibraryPlace::ShuffledIn`).
fn shuffle_them_in(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<ObjectId> = [0, 1].into_iter().filter_map(|i| targets::object(g, item, i)).collect();
    g.put_into_library(&cards, LibraryPlace::ShuffledIn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::draw_then_discard::<2, 2>, "I \u{2014} Draw two cards, then discard two cards."),
        chapter(
            2,
            effects::return_first_target_to_hand,
            "II \u{2014} Return up to one target artifact, instant, or sorcery card from your graveyard to your hand.",
        )
        .targets(&[up_to_one(artifact_instant_or_sorcery_card_in_your_graveyard)]),
        chapter(
            3,
            shuffle_them_in,
            "III \u{2014} Choose up to two target cards in graveyards. The owners of those cards shuffle them into their libraries.",
        )
        .targets(&[up_to(2, targets::CARD_IN_A_GRAVEYARD.filter)]),
    ],
    ..oracle::THOSE_OLD_SCIENTISTS
};
