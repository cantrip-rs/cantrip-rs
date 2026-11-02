use cantrip_api::*;
use cantrip_mechanics::copy;
use cantrip_oracle as oracle;

/// "You may have this creature enter as a copy of any nonland permanent on the battlefield." (CR 608.2).
fn have_creature_enter_as_copy_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    const ASK_0: Question = Question::new(
        "CopyAsEnters",
        "enter as a copy?",
        "You choose what it copies next. If you don't, it enters as itself",
        "Choose a copy",
        "Enter as itself",
    );
    let candidates_0: Vec<ObjectId> = ({
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| !(q.chars(id).is(CardType::Land)))
            .collect::<Vec<ObjectId>>()
    })
        .into_iter()
        .filter(|&id| id != entering.object)
        .collect();
    let Some(chosen_0) = copy::choose_copy_of_one_of(g, entering, &candidates_0, &ASK_0)?
    else { return Ok(()) };
    entering.copy = {
        let q: &dyn Query = g;
        Some(chosen_0).map(|original: ObjectId| q.copiable_values(original))
    };
    vec![chosen_0];
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters_as_copy: Some(have_creature_enter_as_copy_enters),
    ..oracle::CLEVER_IMPERSONATOR
};
