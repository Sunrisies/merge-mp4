#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[route("/")]
    DogView,
    #[route("/analyze")]
    Mp4AnalyzerView,
}
