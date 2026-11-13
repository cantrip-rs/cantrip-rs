use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_mechanics::backup;
use cantrip_oracle as oracle;

/// Backup 1 (CR 702.165a): put a +1/+1 counter on target creature; if that's another creature, it gains flying and the attack ability, fixed when backup triggers (CR 702.165d).
fn backup(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    backup::resolve(g, item, 1, &[Keyword::Flying], Some(0))
}

/// "Target nonland permanent card with mana value X or less from your graveyard, where X is this creature's power": the
/// power of the creature whose ability this is (CR 113.7a), the Scalelord or the creature backup gave the trigger to.
fn permanent_card_up_to_power(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    if !targets::nonland_permanent_card_in_your_graveyard(q, asker, t) {
        return false;
    }
    let Some(power) = q.chars(asker.source).power else { return false };
    q.mana_value(id) as i32 <= power as i32
}

const ATTACK_TEXT: &str = "Whenever this creature attacks, return target nonland permanent card with mana value X or less from your graveyard to the battlefield, where X is this creature's power.";

const ATTACK: AbilityDef = triggered(cantrip_mechanics::triggers::THIS_ATTACKS, effects::return_first_target_to_battlefield, ATTACK_TEXT)
    .targets(&[target(permanent_card_up_to_power)]);

pub static CARD: CardDef = CardDef {
    abilities: &[
        backup::trigger("When this creature enters, put a +1/+1 counter on target creature. If that's another creature, it gains the following abilities until end of turn.", backup),
        ATTACK,
    ],
    delayed: &[ATTACK],
    ..oracle::GUARDIAN_SCALELORD
};
