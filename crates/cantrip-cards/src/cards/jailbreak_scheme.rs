use cantrip_api::*;
use cantrip_mechanics::{effects, library, spree, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on target creature. It can't be blocked this turn." (CR 608.2c): two sentences about the one target, the
/// counter first, then the unblockability; both do nothing if it is no longer a legal target (CR 608.2b).
fn counter_then_cant_be_blocked(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counters_on_target::<1>(g, item)?;
    g.end_sentence();
    effects::target_creature_cant_be_blocked_this_turn(g, item)
}

const MODES: &[ModeDef] = &[
    mode(counter_then_cant_be_blocked, "+ {3} \u{2014} Put a +1/+1 counter on target creature. It can't be blocked this turn.")
        .targets(&[targets::CREATURE])
        .cost(mana("{3}")),
    mode(
        library::owner_puts_first_target_on_top_or_bottom,
        "+ {2} \u{2014} Target artifact or creature's owner puts it on their choice of the top or bottom of their library.",
    )
    .targets(&[targets::ARTIFACT_OR_CREATURE])
    .cost(mana("{2}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::JAILBREAK_SCHEME };
