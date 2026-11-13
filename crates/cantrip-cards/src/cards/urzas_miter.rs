use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// An artifact you control goes to a graveyard without being sacrificed (CR 603.2, 701.21a).
fn artifact_not_sacrificed(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    if matches!(event, Event::ZoneChange { reason: ZoneReason::Sacrifice, .. }) {
        return None;
    }
    triggers::type_you_control_to_graveyard::<{ CardType::Artifact as u8 }>(q, asker, event)
}

/// You may pay {3}; if you do, draw a card (CR 118.12, 121.1).
fn pay_to_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    static QUESTION: Question = Question::new("Pay", "pay {cost} to draw a card?", "", "Pay {cost}", "Decline");
    if g.may_pay(item.controller, item.object, item.object, cost("{3}"), &QUESTION)? {
        g.end_sentence();
        g.draw(item.controller, 1);
        g.end_sentence();
    } else {
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, artifact_not_sacrificed),
        pay_to_draw,
        "Whenever an artifact you control is put into a graveyard from the battlefield, if it wasn't sacrificed, you may pay {3}. If you do, draw a card.",
    )],
    ..oracle::URZAS_MITER
};
