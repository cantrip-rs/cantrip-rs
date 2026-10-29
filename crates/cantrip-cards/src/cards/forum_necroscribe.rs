use cantrip_api::*;
use cantrip_mechanics::triggers::{ward_discard, REPARTEE};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

const WARD: &str = "Ward\u{2014}Discard a card. (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player discards a card.)";

/// "Return target creature card from your graveyard to the battlefield": the target is chosen when the ability goes on the
/// stack, so it can be a card the spell put there; with none to choose, the ability is removed from the stack (CR 603.3d).
pub static CARD: CardDef = CardDef {
    abilities: &[
        ward_discard(WARD),
        triggered(
            REPARTEE,
            effects::return_first_target_to_battlefield,
            "Repartee \u{2014} Whenever you cast an instant or sorcery spell that targets a creature, return target creature card from your graveyard to the battlefield.",
        )
        .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
    ],
    ..oracle::FORUM_NECROSCRIBE
};
