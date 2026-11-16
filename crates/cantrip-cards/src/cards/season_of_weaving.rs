use cantrip_api::*;
use cantrip_mechanics::{choose, effects, select};
use cantrip_oracle as oracle;

/// {P} - "Draw a card."
fn draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)
}

/// {P}{P} - "Choose an artifact or creature you control. Create a token that's a copy of it." Chosen as this mode resolves,
/// not targeted (CR 115.1, 608.2d), so hexproof and the like don't stop it; with none to choose, nothing happens. The token
/// copies its copiable values (CR 707.2) and enters under the Season's controller (CR 111.1).
fn copy_an_artifact_or_creature_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let eligible: Vec<ObjectId> = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| q.controller(id) == you && (q.chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Creature)))
            .collect()
    };
    let rule = select::artifact_or_creature_permanents(select::exactly(1)).called("artifact or creature you control");
    let Some(&chosen) = choose::select(g, you, item.object, Purpose::CopyWhich, eligible, rule)?.first() else { return Ok(()) };
    let values = g.copiable_values(chosen);
    g.create_token_copies_of(values, you, 1)?;
    Ok(())
}

/// {P}{P}{P} - "Return each nonland, nontoken permanent to its owner's hand." Every one at once, its controller's and the
/// opponents' alike (CR 608.2c); the tokens, which stay, include the copies an earlier {P}{P} made, since the modes run in
/// printed order (CR 700.2d).
fn return_each_nonland_nontoken_permanent(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let returned: Vec<ObjectId> = {
        let q: &dyn Query = g;
        q.state().battlefield().iter().copied().filter(|&id| !q.chars(id).is(CardType::Land) && !q.obj(id).token).collect()
    };
    g.return_to_hand(&returned);
    Ok(())
}

static MODES: ModalDef = modes(&[
    mode(draw_a_card, "{P} \u{2014} Draw a card.").pawprints(1),
    mode(
        copy_an_artifact_or_creature_you_control,
        "{P}{P} \u{2014} Choose an artifact or creature you control. Create a token that's a copy of it.",
    )
    .pawprints(2),
    mode(return_each_nonland_nontoken_permanent, "{P}{P}{P} \u{2014} Return each nonland, nontoken permanent to its owner's hand.")
        .pawprints(3),
])
.pawprints(5);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODES), ..oracle::SEASON_OF_WEAVING };
