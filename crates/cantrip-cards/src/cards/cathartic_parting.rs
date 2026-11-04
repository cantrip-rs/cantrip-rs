use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("ShuffleIntoLibrary", "shuffle the targeted cards from your graveyard into your library?", "If you don't, they stay in your graveyard", "Shuffle them in", "Leave them");

/// The owner of the target permanent shuffles it into their library, then you may shuffle up to four target cards
/// from your graveyard into your library. A permanent that is no longer a legal target does nothing and has no owner
/// to shuffle (CR 608.2b); the second sentence is a "you may" asked as it resolves, and a yes shuffles the caster's
/// library even when no card of it remains a legal target (CR 701.24c-d).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::shuffle_first_target_into_owners_library(g, item)?;

    let you = item.controller;
    let cards: Vec<ObjectId> = (1..item.target_count as usize).filter_map(|slot| targets::object(g, item, slot)).collect();
    if !effects::ask_may(g, you, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    g.put_into_library(&cards, LibraryPlace::Top);
    g.shuffle(you);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[ targets::ARTIFACT_OR_ENCHANTMENT_AN_OPPONENT_CONTROLS, up_to(4, targets::any_card_in_your_graveyard), ], effect),
    ..oracle::CATHARTIC_PARTING
};
