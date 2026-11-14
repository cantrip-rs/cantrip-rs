use cantrip_api::*;
use cantrip_mechanics::triggers::this_or_another_of_subtype_you_control_enters;
use cantrip_mechanics::{blight, lands, untapped_subtype_you_control};
use cantrip_oracle as oracle;

/// "Each opponent blights 1" (CR 701.68a): each opponent, in turn order, puts a -1/-1 counter on a creature they control
/// of their choice; one who controls no creature can't (CR 701.68b).
fn each_opponent_blights(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    for opponent in opponents {
        blight::in_effect(g, opponent, item.object, 1)?;
    }
    Ok(())
}

fn proliferate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.proliferate(item.controller, item.object)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, this_or_another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Elf) }>),
            each_opponent_blights,
            "Whenever this creature or another Elf you control enters, each opponent blights 1. (They each put a -1/-1 counter on a creature they control.)",
        ),
        ability(
            tap_untapped(3, untapped_subtype_you_control::<{ lands::sub(Subtype::Elf) }>),
            proliferate,
            "Tap three untapped Elves you control: Proliferate. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    ..oracle::HIGH_PERFECT_MORCANT
};
