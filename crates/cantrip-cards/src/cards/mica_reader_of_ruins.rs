use cantrip_api::*;
use cantrip_mechanics::triggers::{self, ward_life};
use cantrip_mechanics::{copy, effects, select};
use cantrip_oracle as oracle;

/// "You may sacrifice an artifact. If you do, copy that spell. You may choose new targets for the copy" (CR 608.2c-d, 701.21a,
/// 707.10): the sacrifice is chosen as the ability resolves, not paid as a cost, and the copy (not cast) is made only if an
/// artifact was sacrificed, from the spell's last-known information if it has left the stack (CR 608.2h).
fn sacrifice_an_artifact_to_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::may_sacrifice(g, item.controller, item.object, select::artifact_cards(select::up_to(1)))?.is_empty() {
        return Ok(());
    }
    copy::copy_the_cast_spell(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward_life::<3>("Ward\u{2014}Pay 3 life. (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays 3 life.)"),
        triggered(
            triggers::INSTANT_OR_SORCERY_YOU_CAST,
            sacrifice_an_artifact_to_copy,
            "Whenever you cast an instant or sorcery spell, you may sacrifice an artifact. If you do, copy that spell and you may choose new targets for the copy.",
        ),
    ],
    ..oracle::MICA_READER_OF_RUINS
};
