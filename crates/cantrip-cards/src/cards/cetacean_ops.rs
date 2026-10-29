use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<1>("Ward {1} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)"),
        triggered(triggers::NONCREATURE_SPELL_YOU_CAST, plus_one_counter_on_this, "Whenever you cast a noncreature spell, put a +1/+1 counter on this creature."),
    ],
    ..oracle::CETACEAN_OPS
};
