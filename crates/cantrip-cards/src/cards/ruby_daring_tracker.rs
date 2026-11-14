use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, ferocious};
use cantrip_oracle as oracle;

/// "...while you control a creature with power 4 or greater": asked as the attack is declared, as part of the trigger event, and
/// never again as the ability resolves (this is no intervening "if", CR 603.4). Ruby's own power counts: with the +2/+2 not yet
/// given, she needs another creature (or an earlier pump) to be at 4 or more.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS,
            effects::this_gets::<2, 2>,
            "Whenever Ruby attacks while you control a creature with power 4 or greater, Ruby gets +2/+2 until end of turn.",
        )
        .only_if(ferocious::you_control_asked),
        mana_ability(TAP, add_one_of(&[Color::Red, Color::Green]), "{T}: Add {R} or {G}."),
    ],
    ..oracle::RUBY_DARING_TRACKER
};
