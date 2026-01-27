#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[route("/")]
    DogView,
}
