use cantrip_api::*;
use cantrip_mechanics::{lands, tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever you cast a spell during your turn other than your first spell that turn" (CR 601.2i, 603.2): a spell its controller
/// casts on their own turn that is the second or later of the turn (`Event::SpellCast::nth`, counted from the turn's start, so
/// the spells cast before Geralf arrived count toward "first").
fn you_cast_a_spell_other_than_your_first_during_your_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { player, nth, .. } if *player == me.controller && *nth >= 2 && q.state().turn.active == me.controller => Fired::PLAIN,
        _ => None,
    }
}

/// "Put a +1/+1 counter on it for each other Zombie that entered the battlefield under your control this turn" (CR 122.1a, 603.6a):
/// the Zombie that entered (the trigger's subject), the Zombies counted from the turn's history as each entered (the controller
/// and subtypes `Event::Entered` holds, tokens included, whatever has happened to them since) other than it, the Zombie read
/// through the words of Geralf's text (CR 612.2). The count is made as the ability resolves; a Zombie that has left the battlefield gets nothing (CR 400.7).
fn plus_one_counters_for_each_other_zombie_that_entered(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(zombie) = item.subject.object() else { return Ok(()) };
    let word = g.words(item.object).subtype(Subtype::Zombie);
    let others = g.history().entered().filter(|e| e.permanent != zombie && e.controller == item.controller && e.chars(g).has_subtype(word)).count();
    g.add_counters(zombie, CounterKind::PlusOne, others as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, you_cast_a_spell_other_than_your_first_during_your_turn),
            |g, item| {
                tokens::create(g, item.controller, &tokens::ZOMBIE_ROGUE, 1)?;
                Ok(())
            },
            "Whenever you cast a spell during your turn other than your first spell that turn, create a 2/2 blue and black Zombie Rogue creature token.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::of_subtype_you_control_enters::<{ lands::sub(Subtype::Zombie) }>),
            plus_one_counters_for_each_other_zombie_that_entered,
            "Whenever a Zombie you control enters, put a +1/+1 counter on it for each other Zombie that entered the battlefield under your control this turn.",
        ),
    ],
    ..oracle::GERALF_THE_FLESHWRIGHT
};

pub static EXTRA: &[&CardDef] = &[&tokens::ZOMBIE_ROGUE];
