use cantrip_api::*;
use cantrip_mechanics::combat::create_attacking_of_your_choice;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::tokens::GNOME;
use cantrip_oracle as oracle;

/// "Whenever you attack with one or more non-Gnome creatures" (CR 508.1m, 603.2c): a creature its controller declares as an attacker
/// that is not a Gnome, read through the words of the source's text (CR 612). The attackers declared together are one batch
/// (`once_per_batch`), each a subject.
fn you_attack_with_a_non_gnome_creature(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if q.controller(*attacker) == me.controller && !q.chars(*attacker).has_subtype(q.words(me.source).subtype(Subtype::Gnome)) => {
            Fired::about(Target::Object(*attacker))
        }
        _ => None,
    }
}

/// "Put a +1/+1 counter on Anim Pakal, then create X 1/1 colorless Gnome artifact creature tokens that are tapped and attacking, where X
/// is the number of +1/+1 counters on Anim Pakal" (CR 122.1, 508.4, 608.2h): the counter goes on only while Anim Pakal is still the
/// permanent that triggered (CR 400.7); X counts its counters after that, or as it last was on the battlefield (CR 603.10a) when
/// it has left, and the Gnomes come regardless. Their controller chooses what each attacks, not necessarily what the
/// attackers do, and the ones attacking the same thing are created together (CR 101.4).
fn counter_then_gnomes(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    plus_one_counter_on_this(g, item)?;
    let count = g.obj(item.object).counters(CounterKind::PlusOne) as usize;
    let gnome = g.card_id(&GNOME);
    create_attacking_of_your_choice(g, item.controller, item.object, gnome, count, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ATTACKS, you_attack_with_a_non_gnome_creature),
        counter_then_gnomes,
        "Whenever you attack with one or more non-Gnome creatures, put a +1/+1 counter on Anim Pakal, then create X 1/1 colorless Gnome artifact creature tokens that are tapped and attacking, where X is the number of +1/+1 counters on Anim Pakal.",
    )
    .once_per_batch()],
    ..oracle::ANIM_PAKAL_THOUSANDTH_MOON
};

pub static EXTRA: &[&CardDef] = &[&GNOME];
