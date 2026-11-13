use cantrip_api::*;
use cantrip_mechanics::{effects, lands, tokens};
use cantrip_oracle as oracle;

/// "Whenever another nontoken Hero you control enters" (CR 603.6a): a permanent other than this one, a card and not a token,
/// under your control as it entered, that is a Hero, the type read through the words of the source's text (CR 612). The
/// entering permanent is the trigger's subject.
fn another_nontoken_hero_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if *new != me.source
                && to.is(ZoneKind::Battlefield)
                && !q.obj(*new).token
                && q.controller(*new) == me.controller
                && q.chars(*new).has_subtype(lands::word(q, me.source, lands::sub(Subtype::Hero))) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

/// "Choose one" as the trigger goes on the stack (CR 603.3c, 700.2a): "Create a 1/1 white Soldier creature token" (CR 111.4) or
/// "Creatures you control get +1/+1 until end of turn" (CR 611.2a).
static MODAL: ModalDef = modes(&[
    mode(tokens::create_a_soldier, "Create a 1/1 white Soldier creature token."),
    mode(pump_creatures_you_control, "Creatures you control get +1/+1 until end of turn."),
]);

fn pump_creatures_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_creatures_you_control(g, item, 1, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        when_asked(EventMask::ZONE_CHANGE, another_nontoken_hero_you_control_enters),
        &MODAL,
        "Whenever another nontoken Hero you control enters, choose one \u{2014}\n\u{2022} Create a 1/1 white Soldier creature token.\n\u{2022} Creatures you control get +1/+1 until end of turn.",
    )],
    ..oracle::BLACK_PANTHER_VANGUARD
};

pub static EXTRA: &[&CardDef] = &[&tokens::SOLDIER];
