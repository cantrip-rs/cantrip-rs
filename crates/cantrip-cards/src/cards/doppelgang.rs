use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "For each of X target permanents, create X tokens that are copies of that permanent." (CR 107.3, 608.2b, 707.2, 111.1): X is
/// the number chosen as it was cast, for its three {X} and for the number of targets. A target that is no longer legal is
/// not copied (CR 608.2b); each of the others gets X tokens, copies of it as it is when the spell resolves (CR 707.2),
/// and all of it is one sentence, one instruction (CR 608.2c): the tokens enter together, so each sees the others enter (CR 603.6a).
fn copy_each_target_x_times(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut copies: Vec<(Copiable, PlayerId)> = Vec::new();
    for permanent in targets::legal_objects(g, item) {
        let copiable = g.copiable_values(permanent);
        for _ in 0..item.x {
            copies.push((copiable.clone(), item.controller));
        }
    }
    g.create_token_copies_together(&copies)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PERMANENT.counted()], copy_each_target_x_times),
    ..oracle::DOPPELGANG
};
