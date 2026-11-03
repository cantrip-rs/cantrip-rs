use cantrip_api::*;
use cantrip_mechanics::{tokens, scry};
use cantrip_oracle as oracle;

/// "When this artifact enters and when you sacrifice it" (CR 603.2).
fn when_artifact_enters_when_sacrifice(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::ZoneChange { new, to, old, from, reason, .. } = event else { return None };
    let id = *new;
    let subject = Target::Object(id);
    if to.is(ZoneKind::Battlefield) && asker.source == id {
        return Fired::about(subject);
    }
    let ZoneReason::Sacrifice = reason else { return None };
    let id = *old;
    let player = q.controller(id);
    let subject = Target::Object(id);
    if from.is(ZoneKind::Battlefield) && asker.controller == player && asker.source == id
    {
        return Fired::about(subject);
    }
    None
}

/// "When this artifact enters and when you sacrifice it, create a 1/1 white Rabbit creature token and scry 1." (CR 608.2, 111.1).
fn create_1_1_white_rabbit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_0 = Vec::new();
    let def_0_0 = g.card_id(&tokens::RABBIT);
    tokens_0.extend(std::iter::repeat((def_0_0, item.controller)).take(1));
    let words_0 = g.words(item.object);
    g.create_tokens_worded(&tokens_0, false, None, words_0)?;
    g.end_sentence();
    scry(g, item.controller, item.object, 1)?;
    Ok(())
}

/// "{2}, {T}, Sacrifice this artifact: You gain 3 life." (CR 608.2, 119.3).
fn gain_3_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::ZONE_CHANGE, when_artifact_enters_when_sacrifice),
                create_1_1_white_rabbit,
                "When this artifact enters and when you sacrifice it, create a 1/1 white Rabbit creature token and scry 1.",
            )
            .on_sacrifice(),
        ability(
            cost("{2}").and(SACRIFICE_THIS).and(TAP),
            gain_3_life,
            "{2}, {T}, Sacrifice this artifact: You gain 3 life.",
        ),
    ],
    ..oracle::CARROT_CAKE
};

pub static EXTRA: &[&CardDef] = &[&tokens::RABBIT];
