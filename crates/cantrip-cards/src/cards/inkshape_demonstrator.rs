use cantrip_api::*;
use cantrip_mechanics::effects::this_gets_and_gains_lifelink;
use cantrip_mechanics::triggers::{ward, REPARTEE};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        triggered(
            REPARTEE,
            this_gets_and_gains_lifelink::<1, 0>,
            "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, this creature gets +1/+0 and gains lifelink until end of turn.",
        ),
    ],
    ..oracle::INKSHAPE_DEMONSTRATOR
};
