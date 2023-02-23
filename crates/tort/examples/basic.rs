use tort::app::App;
use tort::DefaultPlugins;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .run();
}