use cantrip_api::*;
use cantrip_mechanics::{effects, mana, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped_with_stun_counters::<3>),
    abilities: &[
        triggers::ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        ability(cost("{1}{U}"), effects::untap_self, "{1}{U}: Untap this creature."),
    ],
    ..oracle::SLEEP_CURSED_FAERIE
};
