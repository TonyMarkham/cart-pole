use axum::response::Html;

/// The page at `/`, kept beside this file and compiled in.
const HELLO_PAGE: &str = include_str!("hello.html");

/// The hello page: a link to each of the server's pages, so knowing the Orin's
/// address is enough to find them.
pub(crate) async fn hello() -> Html<&'static str> {
    Html(HELLO_PAGE)
}
