use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::this_dealt_damage};
use cantrip_oracle as oracle;

/// "Draw X cards" (CR 107.3a, 121.1): X is the one announced for the ability, whose cost counted it twice ({X}{X}, CR 107.3).
fn draw_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, u32::from(item.x));
    Ok(())
}

/// "Put a +1/+1 counter on him. If he's attacking, untap him and there is an additional combat phase after this phase."
/// (CR 122.1, 701.26b, 500.8): whether he is attacking is judged as this resolves (CR 608.2c), which is the damage's own combat
/// or any other; an attacking creature stays attacking when it untaps (CR 506.4), and the new combat phase begins after the one
/// under way, so he may attack again in it.
fn enrage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    g.end_sentence();
    if g.is_attacking(item.object) {
        g.untap(item.object);
        g.add_combat_phase();
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::DAMAGE, this_dealt_damage),
        enrage,
        "Enrage \u{2014} Whenever The Incredible Hulk is dealt damage, put a +1/+1 counter on him. If he's attacking, untap him and there is an additional combat phase after this phase.",
    )
    .sums_per_subject()],
    ..oracle::THE_INCREDIBLE_HULK
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{X}{X}, {T}"), draw_x, "{X}{X}, {T}: Draw X cards. Activate only as a sorcery.").sorcery_speed(),
        ability(cost("{2}{R}{R}{G}{G}"), effects::transform_self, "{2}{R}{R}{G}{G}: Transform Bruce Banner. Activate only as a sorcery.").sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::BRUCE_BANNER
};
