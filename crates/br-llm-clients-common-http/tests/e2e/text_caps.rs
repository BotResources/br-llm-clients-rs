use br_llm_clients_common_http::{
    ApiKeyAuthorizer, AuthorizationRequest, AuthorizedHeader, Authorizer, AuthorizerError,
    BoxFuture, Endpoint, NoAuthorizer, Request, Secret, SendError,
};

use crate::support::{client, guarded};

fn shown(error: &SendError) -> [String; 3] {
    [
        error.to_string(),
        format!("{error:?}"),
        format!("{error:#?}"),
    ]
}

fn assert_short(error: &SendError) {
    for text in shown(error) {
        assert!(
            text.chars().count() <= 512,
            "{} chars: {text}",
            text.chars().count()
        );
    }
}

fn unlistened(path: &str) -> Endpoint {
    crate::support::assert_port_one_is_closed();
    Endpoint::plain_http(&format!("http://127.0.0.1:1{path}")).expect("endpoint")
}

#[tokio::test]
async fn given_a_secret_that_straddles_the_cut_then_neither_part_of_it_is_in_the_text() {
    let secret = "sk-CANARY-0123456789abcdefXYZ";
    for offset in [300usize, 360, 380, 395, 399, 400, 405, 420] {
        let path = format!("/{}{}{}", "p".repeat(offset), secret, "/q".repeat(100));
        let authorizer = ApiKeyAuthorizer::new("x-api-key", Secret::new(secret).expect("secret"))
            .expect("authorizer");
        let error = guarded(client().send(Request::new(unlistened(&path), "b"), &authorizer))
            .await
            .unwrap_err();
        assert_short(&error);
        for text in shown(&error) {
            assert!(!text.contains("sk-CANARY"), "offset {offset}: {text}");
            assert!(!text.contains("CANARY-0123"), "offset {offset}: {text}");
        }
    }
}

#[tokio::test]
async fn given_a_two_character_secret_repeated_in_a_long_path_then_the_text_stays_short() {
    let path = format!("/{}", "ab/".repeat(2700));
    let authorizer =
        ApiKeyAuthorizer::new("x-api-key", Secret::new("ab").expect("secret")).expect("authorizer");
    let error = guarded(client().send(Request::new(unlistened(&path), "b"), &authorizer))
        .await
        .unwrap_err();
    assert_short(&error);
}

#[tokio::test]
async fn given_the_longest_endpoint_then_every_failure_text_stays_short() {
    let path = format!("/{}", "x".repeat(8000));
    let error = guarded(client().send(Request::new(unlistened(&path), "b"), &NoAuthorizer))
        .await
        .unwrap_err();
    assert_short(&error);
    assert!(matches!(error, SendError::Connect { .. }));
}

struct Loud(AuthorizerError);

impl Authorizer for Loud {
    fn authorize<'a>(
        &'a self,
        _request: &'a AuthorizationRequest<'a>,
    ) -> BoxFuture<'a, Result<Vec<AuthorizedHeader>, AuthorizerError>> {
        let error = self.0.clone();
        Box::pin(async move { Err(error) })
    }
}

#[tokio::test]
async fn given_an_authorizer_error_with_a_huge_text_then_it_is_cut_and_has_no_control_characters() {
    for error in [
        AuthorizerError::fetch_failed(format!("X\n\"\\{}", "y".repeat(5000))),
        AuthorizerError::unavailable("é".repeat(3000)),
    ] {
        let failure = guarded(client().send(Request::new(unlistened("/"), "b"), &Loud(error)))
            .await
            .unwrap_err();
        assert!(matches!(failure, SendError::Authorizer(_)), "{failure:?}");
        assert_short(&failure);
        assert!(!failure.to_string().contains('\n'));
    }
}
