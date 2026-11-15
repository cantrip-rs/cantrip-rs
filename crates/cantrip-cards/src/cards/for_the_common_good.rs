use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Create X tokens that are copies of target token you control. Then tokens you control gain indestructible until your next turn.
/// You gain 1 life for each token you control." (CR 707.2, 111.1, 611.2c, 608.2c): nothing happens if the target is no longer
/// legal (CR 608.2b). The copies are all created at once, then the tokens the controller controls - the copies too - gain
/// indestructible, and no token that enters later does (CR 611.2c). Last, the life is one for each token the controller controls
/// then, counted after the keyword is granted.
fn copies_then_tokens_gain_indestructible_then_gain_life_for_each_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(original) = targets::object(g, item, 0) else { return Ok(()) };
    g.create_token_copies(original, item.controller, u32::from(item.x))?;
    g.end_sentence();
    let tokens = tokens_you_control(g, item.controller);
    instruction(g, |g| {
        for &id in &tokens {
            g.add_effect(
                item.object,
                id,
                EffectKind::AddKeywords(Keywords::of(&[Keyword::Indestructible])),
                Duration::UntilNextTurnOf(item.controller),
            );
        }
    });
    let count = tokens_you_control(g, item.controller).len() as i32;
    g.gain_life(item.controller, count);
    Ok(())
}

/// The tokens `you` control on the battlefield.
fn tokens_you_control(g: &dyn Game, you: PlayerId) -> Vec<ObjectId> {
    g.permanents_of(you).into_iter().filter(|&id| g.obj(id).token).collect()
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::TOKEN_YOU_CONTROL], copies_then_tokens_gain_indestructible_then_gain_life_for_each_token),
    ..oracle::FOR_THE_COMMON_GOOD
};
