use br_llm_messages::{BlockId, ErrorCode, TextBlock};

use crate::renew::{RenewError, collide};
use crate::sequential_id_source::identity_uuid;

#[test]
fn given_the_error_of_a_reused_identity_when_classified_then_it_is_a_collision() {
    let id = BlockId::from_uuid(identity_uuid(7)).unwrap();
    let first = TextBlock::new(id, "a").unwrap();
    let second = TextBlock::new(id, "b").unwrap();
    let error = br_llm_messages::ModelResponse::new(
        crate::fixtures::entry_id(),
        vec![first.into(), second.into()],
        br_llm_messages::ResponseEnding::finished(),
        br_llm_messages::ResponseMetadata::new(),
    )
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(collide::<()>(Err(error)), Err(RenewError::Collision));
}

#[test]
fn given_any_other_rebuild_error_when_classified_then_it_is_not_a_collision() {
    let error = TextBlock::new(crate::fixtures::block_id(), "").unwrap_err();
    assert_ne!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(
        collide::<()>(Err(error.clone())),
        Err(RenewError::Inconsistent(error))
    );
}
