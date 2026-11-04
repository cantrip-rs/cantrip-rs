use cantrip_api::*;
use cantrip_mechanics::triggers::{self, THIS_ENTERS};
use cantrip_mechanics::{effects, library};
use cantrip_oracle as oracle;

/// "Whenever you play a land from exile or cast a spell from exile" (CR 305.1, 601.2a, 603.2): a spell its controller cast
/// from exile (a copy cast from exile counts), or a land they played from exile, which is a land play (a land put onto the
/// battlefield from exile by an effect is not played).
fn you_play_a_land_or_cast_a_spell_from_exile(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, from, reason: ZoneReason::PlayLand, .. } if from.is(ZoneKind::Exile) && q.obj(*new).controller == me.controller => {
            Fired::PLAIN
        }
        _ => triggers::you_cast_a_spell_from_exile(q, me, event),
    }
}

fn put_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask(EventMask::SPELL_CAST.0 | EventMask::ZONE_CHANGE.0), you_play_a_land_or_cast_a_spell_from_exile),
            put_counter,
            "Whenever you play a land from exile or cast a spell from exile, put a +1/+1 counter on Ghost-Spider.",
        ),
        ability(
            remove_counters_among(2, cantrip_mechanics::targets::filters::this_permanent),
            library::exile_top_card_play_this_turn,
            "Remove two counters from Ghost-Spider: Exile the top card of your library. You may play that card this turn.",
        ),
    ],
    ..oracle::GHOST_SPIDER
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            // The permission ends the first time her controller doesn't control her, and her transforming into Ghost-Spider is not that
            // (CR 611.2b, 701.27f: the same object).
            library::exile_top_card_play_while_you_control,
            "When Gwen Stacy enters, exile the top card of your library. You may play that card for as long as you control this creature.",
        ),
        ability(cost("{2}{U}{R}{W}"), effects::transform_self, "{2}{U}{R}{W}: Transform Gwen Stacy. Activate only as a sorcery.").sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::GWEN_STACY
};
