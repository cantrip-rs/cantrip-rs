use cantrip_api::*;
use cantrip_mechanics::{delayed, effects};
use cantrip_oracle as oracle;

/// "Until end of turn" (CR 603.2).
fn watch_1_event(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let id = *spell;
    let subject = Target::Object(id);
    if asker.controller == *player
        && q.chars(id).colors.has(q.words(asker.source).color(Color::Black))
    {
        return Fired::about(subject);
    }
    None
}

/// "{1}{R}{R}: Until end of turn, whenever you cast a black spell, put a +1/+1 counter on this creature." (CR 608.2).
fn whenever_cast_black_spell_put(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    delayed::this_turn(g, item, item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}{R}{R}"),
            whenever_cast_black_spell_put,
            "{1}{R}{R}: Until end of turn, whenever you cast a black spell, put a +1/+1 counter on this creature.",
        ),
    ],
    delayed: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, watch_1_event),
            effects::plus_one_counter_on_this,
            "Until end of turn, whenever you cast a black spell, put a +1/+1 counter on this creature.",
        ),
    ],
    ..oracle::MOUNTAIN_TITAN
};
