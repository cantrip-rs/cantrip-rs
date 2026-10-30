use cantrip_api::*;
use cantrip_mechanics::blight;
use cantrip_oracle as oracle;

/// "Whenever this creature or another Elf you control enters" (CR 603.6a): this permanent, or another Elf - a token too -
/// under your control as it entered. "Elf" is a word of its text (CR 612).
fn this_or_another_elf_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield)
                && (*new == me.source
                    || (q.controller(*new) == me.controller && q.chars(*new).has_subtype(q.words(me.source).subtype(Subtype::Elf)))) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Each opponent blights 1" (CR 701.68a): each opponent, in turn order, puts a -1/-1 counter on a creature they control
/// of their choice; one who controls no creature can't (CR 701.68b).
fn each_opponent_blights(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    for opponent in opponents {
        blight::in_effect(g, opponent, item.object, 1)?;
    }
    Ok(())
}

/// "Tap three untapped Elves you control" (CR 602.2b, 118.3): any untapped Elf its controller controls, this one and one
/// that came under their control this turn included (CR 302.6 limits only a creature's own {T} abilities, and this is not
/// {T} but the tapping of permanents as a cost).
fn untapped_elf_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.controller(id) == asker.controller && !q.obj(id).tapped && q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Elf))
}

fn proliferate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.proliferate(item.controller, item.object)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, this_or_another_elf_enters),
            each_opponent_blights,
            "Whenever this creature or another Elf you control enters, each opponent blights 1. (They each put a -1/-1 counter on a creature they control.)",
        ),
        ability(
            tap_untapped(3, untapped_elf_you_control),
            proliferate,
            "Tap three untapped Elves you control: Proliferate. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    ..oracle::HIGH_PERFECT_MORCANT
};
