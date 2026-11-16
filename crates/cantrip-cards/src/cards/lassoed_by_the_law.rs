use cantrip_api::*;
use cantrip_mechanics::{exile_target_until_leaves_effect, targets, tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Two "When this enchantment enters" triggers (CR 603.2): the first exiles target nonland permanent an opponent controls
/// until this enchantment leaves the battlefield (CR 610.3; with the enchantment gone before it resolves, nothing is exiled at
/// all, CR 610.3b), the second creates the Mercenary token. They are separate abilities, so the controller orders them.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            exile_target_until_leaves_effect,
            "When this enchantment enters, exile target nonland permanent an opponent controls until this enchantment leaves the battlefield.",
        )
        .targets(&[targets::NONLAND_PERMANENT_AN_OPPONENT_CONTROLS]),
        triggered(
            THIS_ENTERS,
            tokens::create_a_mercenary,
            "When this enchantment enters, create a 1/1 red Mercenary creature token with \"{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.\"",
        ),
    ],
    ..oracle::LASSOED_BY_THE_LAW
};

pub static EXTRA: &[&CardDef] = &[&tokens::MERCENARY];
