use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// A card in exile that has an Adventure (CR 715.2): the printed card, not what it is in exile as (a card exiled on an adventure
/// or exiled some other way both count, and an exiled token does not exist as a card). `yours` picks the cards the controller owns
/// or the ones it does not (CR 108.3: the owner, whoever exiled it). A card that has left exile is a new object (CR 400.7) the
/// core no longer offers or holds legal (`target_legal`).
fn exiled_adventure_card(q: &dyn Query, asker: &Asker, t: Target, yours: bool) -> bool {
    matches!(t, Target::Object(id) if {
        let obj = q.obj(id);
        obj.zone.is(ZoneKind::Exile) && (obj.owner == asker.controller) == yours && library::card_has_an_adventure(q, id)
    })
}

fn exiled_adventure_card_you_own(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    exiled_adventure_card(q, asker, t, true)
}

fn exiled_adventure_card_you_dont_own(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    exiled_adventure_card(q, asker, t, false)
}

/// "Choose one or more" as the trigger goes on the stack (CR 603.3c, 700.2), each chosen mode with its own target (CR 603.3d,
/// 700.2c) and carried out in the printed order (CR 608.2c), each only if its target is still legal as it resolves (CR 608.2b).
/// The first two reach into exile, the third is any player's graveyard, its own controller's included.
static MODAL: ModalDef = modes(&[
    mode(effects::return_first_target_to_hand, "Return target card you own in exile that has an Adventure to your hand.")
        .targets(&[target(exiled_adventure_card_you_own)]),
    mode(library::put_first_target_on_the_bottom, "Put target card you don't own in exile that has an Adventure on the bottom of its owner's library.")
        .targets(&[target(exiled_adventure_card_you_dont_own)]),
    mode(effects::exile_first_target_players_graveyard, "Exile target player's graveyard.").targets(&[targets::PLAYER]),
])
.choose(1, 3);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODAL,
        "When this creature enters, choose one or more \u{2014}\n\u{2022} Return target card you own in exile that has an Adventure to your hand.\n\u{2022} Put target card you don't own in exile that has an Adventure on the bottom of its owner's library.\n\u{2022} Exile target player's graveyard.",
    )],
    ..oracle::SENTINEL_OF_LOST_LORE
};
